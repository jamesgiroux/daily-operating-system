import { useState, useEffect, useCallback, useRef } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";

import type {
  WeekOverview,
  ReadinessCheck,
  WeekAction,
} from "@/types";
import { cn } from "@/lib/utils";
import {
  Calendar,
  Check,
  ChevronRight,
  Play,
  RefreshCw,
  AlertTriangle,
  Sparkles,
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
  { key: "delivering", label: "Deliver", icon: Package },
  { key: "enriching", label: "Enrich", icon: Wand2 },
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format a due date as a relative phrase: "due Wednesday", "1 day overdue" */
function formatDueContext(
  dueDate?: string,
  daysOverdue?: number
): string | null {
  if (!dueDate) return null;

  if (daysOverdue != null && daysOverdue > 0) {
    return daysOverdue === 1 ? "1 day overdue" : `${daysOverdue} days overdue`;
  }

  try {
    const date = new Date(dueDate + "T00:00:00");
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays < 0)
      return `${Math.abs(diffDays)} day${Math.abs(diffDays) !== 1 ? "s" : ""} overdue`;
    if (diffDays === 0) return "due today";
    if (diffDays === 1) return "due tomorrow";
    if (diffDays <= 6)
      return `due ${date.toLocaleDateString("en-US", { weekday: "long" })}`;
    return `due ${date.toLocaleDateString("en-US", { month: "short", day: "numeric" })}`;
  } catch {
    return null;
  }
}

/** Synthesize readiness checks into a one-line summary */
function synthesizeReadiness(checks: ReadinessCheck[]): string {
  const prepNeeded = checks.filter(
    (c) =>
      c.checkType === "no_prep" ||
      c.checkType === "prep_needed" ||
      c.checkType === "agenda_needed"
  ).length;
  const overdueActions = checks.filter(
    (c) => c.checkType === "overdue_action"
  );
  const staleContacts = checks.filter(
    (c) => c.checkType === "stale_contact"
  ).length;

  const parts: string[] = [];
  if (prepNeeded > 0)
    parts.push(
      `${prepNeeded} meeting${prepNeeded !== 1 ? "s" : ""} need prep`
    );
  if (overdueActions.length > 0) {
    const msg = overdueActions[0].message;
    const match = msg.match(/^(\d+)/);
    const count = match ? match[1] : overdueActions.length.toString();
    parts.push(`${count} overdue action${count !== "1" ? "s" : ""}`);
  }
  if (staleContacts > 0)
    parts.push(
      `${staleContacts} stale contact${staleContacts !== 1 ? "s" : ""}`
    );
  return parts.join(" · ");
}

// ---------------------------------------------------------------------------
// Page Component
// ---------------------------------------------------------------------------

export default function WeekPage() {
  const [data, setData] = useState<WeekOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<WorkflowPhase | null>(null);

  const loadWeek = useCallback(async () => {
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
  }, []);

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
          loadWeek();
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
          loadWeek();
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

  // Loading skeleton — briefing-shaped
  if (loading) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="px-8 pt-10 pb-8">
          <div className="mx-auto max-w-2xl space-y-8">
            <div className="space-y-2">
              <Skeleton className="h-8 w-40" />
              <Skeleton className="h-4 w-32" />
            </div>
            <div className="space-y-3">
              <Skeleton className="h-5 w-full" />
              <Skeleton className="h-5 w-full" />
              <Skeleton className="h-5 w-3/4" />
            </div>
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-px w-full" />
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="space-y-1">
                  <Skeleton className="h-4 w-3/5" />
                  <Skeleton className="h-3 w-2/5" />
                </div>
              ))}
            </div>
          </div>
        </div>
      </main>
    );
  }

  // Empty state
  if (!data) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="flex h-full flex-col items-center justify-center text-center">
          {running ? (
            <WorkflowProgress phase={phase ?? "preparing"} />
          ) : (
            <>
              <Calendar className="mb-4 size-12 text-muted-foreground/30" />
              <p className="text-lg font-medium">No weekly briefing yet</p>
              <p className="mt-1 max-w-sm text-sm text-muted-foreground">
                Generate your weekly briefing to see what matters this week.
              </p>
              <Button className="mt-4 gap-1.5" onClick={handleRunWeek}>
                <Play className="size-3.5" />
                Run Weekly Briefing
              </Button>
            </>
          )}
          {error && <ErrorCard error={error} />}
        </div>
      </main>
    );
  }

  const hasNarrative = !!data.weekNarrative;
  const readinessLine =
    data.readinessChecks && data.readinessChecks.length > 0
      ? synthesizeReadiness(data.readinessChecks)
      : null;

  // Merge all commitments: overdue first, then due this week
  const commitments: (WeekAction & { isOverdue: boolean })[] = [];
  if (data.actionSummary) {
    if (data.actionSummary.overdue) {
      for (const a of data.actionSummary.overdue) {
        commitments.push({ ...a, isOverdue: true });
      }
    }
    if (data.actionSummary.dueThisWeekItems) {
      for (const a of data.actionSummary.dueThisWeekItems) {
        commitments.push({ ...a, isOverdue: false });
      }
    }
  }

  const hasPortfolio = data.hygieneAlerts && data.hygieneAlerts.length > 0;

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="px-8 pt-10 pb-8">
          <div className="mx-auto max-w-2xl">
            {/* Header */}
            <div className="flex items-start justify-between">
              <div>
                <h1 className="text-2xl font-semibold tracking-tight">
                  Week {data.weekNumber}
                </h1>
                <p className="text-sm text-muted-foreground">
                  {data.dateRange}
                </p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                className="gap-1.5"
                onClick={handleRunWeek}
                disabled={running}
              >
                {running ? (
                  <>
                    <RefreshCw className="size-3.5 animate-spin" />
                    <span>
                      {phase
                        ? (phaseSteps.find((s) => s.key === phase)?.label ??
                          "Running...")
                        : "Running..."}
                    </span>
                  </>
                ) : (
                  <>
                    <RefreshCw className="size-3.5" />
                    <span>Refresh</span>
                  </>
                )}
              </Button>
            </div>

            {/* Progress stepper — only when no data yet */}
            {running && !hasNarrative && commitments.length === 0 && (
              <div className="mt-6">
                <WorkflowProgress phase={phase ?? "preparing"} />
              </div>
            )}

            {/* The Briefing — narrative, priority, readiness */}
            <div className="mt-8 space-y-6">
              {data.weekNarrative && (
                <p className="text-[1.075rem] leading-[1.8] text-foreground">
                  {data.weekNarrative}
                </p>
              )}

              {data.topPriority && (
                <div className="rounded-lg bg-success/10 border border-success/15 px-4 py-3.5">
                  <div className="flex items-center gap-2 mb-2">
                    <Sparkles className="size-4 shrink-0 text-success" />
                    <span className="text-sm font-semibold text-success">
                      Top Priority
                    </span>
                  </div>
                  <p className="text-sm font-medium text-foreground">
                    {data.topPriority.title}
                  </p>
                  <p className="mt-1 text-sm text-muted-foreground leading-relaxed">
                    {data.topPriority.reason}
                  </p>
                </div>
              )}

              {/* Readiness — synthesized one-liner when narrative exists */}
              {readinessLine && hasNarrative && (
                <p className="text-sm text-muted-foreground">
                  {readinessLine}
                </p>
              )}

              {/* Readiness — enumerated fallback when no narrative */}
              {!hasNarrative &&
                data.readinessChecks &&
                data.readinessChecks.length > 0 && (
                  <div className="space-y-1.5">
                    {[
                      ...data.readinessChecks.filter(
                        (c) => c.severity === "action_needed"
                      ),
                      ...data.readinessChecks.filter(
                        (c) => c.severity === "heads_up"
                      ),
                    ].map((check, i) => (
                      <p
                        key={i}
                        className={cn(
                          "text-sm",
                          check.severity === "action_needed"
                            ? "text-destructive"
                            : "text-muted-foreground"
                        )}
                      >
                        {check.message}
                      </p>
                    ))}
                  </div>
                )}
            </div>

            {/* Divider — transition from briefing to commitments */}
            {commitments.length > 0 && (
              <div className="my-10">
                <div className="h-px bg-border/40" />
              </div>
            )}

            {/* Commitments — unified list, no section header */}
            {commitments.length > 0 && (
              <div className="space-y-1">
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
                      className="group flex items-center gap-3 rounded-md px-3 py-3 transition-colors hover:bg-muted/50"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium leading-snug">
                          {action.title}
                        </p>
                        <p className="mt-0.5 text-xs text-muted-foreground">
                          {action.account && <span>{action.account}</span>}
                          {action.account && dueContext && <span> · </span>}
                          {dueContext && (
                            <span
                              className={cn(
                                action.isOverdue && "text-destructive"
                              )}
                            >
                              {dueContext}
                            </span>
                          )}
                        </p>
                      </div>
                      <ChevronRight className="size-3.5 shrink-0 text-muted-foreground/0 group-hover:text-muted-foreground transition-colors" />
                    </Link>
                  );
                })}
              </div>
            )}

            {/* Divider — transition from commitments to portfolio */}
            {hasPortfolio && (
              <div className="my-10">
                <div className="h-px bg-border/40" />
              </div>
            )}

            {/* Portfolio watch — prose, no widget chrome */}
            {hasPortfolio && (
              <div className="space-y-4">
                {data.hygieneAlerts!.map((alert, i) => (
                  <div key={i} className="text-sm leading-relaxed">
                    <span className="font-medium">{alert.account}</span>
                    {alert.lifecycle && (
                      <span
                        className={cn(
                          "ml-1.5",
                          alert.severity === "critical"
                            ? "text-destructive"
                            : alert.severity === "warning"
                              ? "text-primary"
                              : "text-muted-foreground"
                        )}
                      >
                        · {alert.lifecycle}
                      </span>
                    )}
                    {alert.arr && (
                      <span className="text-muted-foreground">
                        {" "}
                        · {alert.arr}
                      </span>
                    )}
                    <p className="mt-1 text-sm text-muted-foreground leading-relaxed">
                      {alert.issue}
                    </p>
                  </div>
                ))}
              </div>
            )}

            {error && <ErrorCard error={error} />}

            {/* Page end — the briefing is finite */}
            <div className="mt-12 flex items-center gap-3 text-xs text-muted-foreground">
              <div className="h-px flex-1 bg-border/50" />
              <span>End of weekly briefing</span>
              <div className="h-px flex-1 bg-border/50" />
            </div>
          </div>
        </div>
      </ScrollArea>
    </main>
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

function WorkflowProgress({ phase }: { phase: WorkflowPhase }) {
  const [messageIndex, setMessageIndex] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const startTime = useRef(Date.now());
  const messages = useRef(
    [...waitingMessages].sort(() => Math.random() - 0.5)
  );

  const currentStepIndex = phaseSteps.findIndex((s) => s.key === phase);

  useEffect(() => {
    const interval = setInterval(() => {
      setMessageIndex((i) => (i + 1) % messages.current.length);
    }, 6000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime.current) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const formatElapsed = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  };

  return (
    <div className="flex flex-col items-center gap-6 py-8">
      <Sparkles className="size-10 animate-pulse text-primary" />

      <div className="flex items-center gap-2">
        {phaseSteps.map((step, i) => {
          const StepIcon = step.icon;
          const isComplete = i < currentStepIndex;
          const isCurrent = i === currentStepIndex;

          return (
            <div key={step.key} className="flex items-center gap-2">
              {i > 0 && (
                <div
                  className={cn(
                    "h-px w-8",
                    i <= currentStepIndex ? "bg-primary" : "bg-border"
                  )}
                />
              )}
              <div className="flex flex-col items-center gap-1.5">
                <div
                  className={cn(
                    "flex size-8 items-center justify-center rounded-full border-2 transition-colors",
                    isComplete &&
                      "border-primary bg-primary text-primary-foreground",
                    isCurrent && "border-primary bg-primary/10 text-primary",
                    !isComplete &&
                      !isCurrent &&
                      "border-border text-muted-foreground"
                  )}
                >
                  {isComplete ? (
                    <Check className="size-4" />
                  ) : (
                    <StepIcon
                      className={cn("size-4", isCurrent && "animate-pulse")}
                    />
                  )}
                </div>
                <span
                  className={cn(
                    "text-xs font-medium",
                    isCurrent ? "text-primary" : "text-muted-foreground"
                  )}
                >
                  {step.label}
                </span>
              </div>
            </div>
          );
        })}
      </div>

      <p className="max-w-md text-sm italic text-muted-foreground">
        {messages.current[messageIndex]}
      </p>

      <div className="space-y-1 text-center">
        <p className="font-mono text-xs text-muted-foreground/60">
          {formatElapsed(elapsed)}
        </p>
        <p className="text-xs text-muted-foreground/50">
          This runs in the background — feel free to navigate away
        </p>
      </div>
    </div>
  );
}
