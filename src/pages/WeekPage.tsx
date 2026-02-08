import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";

import type { WeekOverview, WeekDay, WeekMeeting, TimeBlock, PrepStatus, AlertSeverity } from "@/types";
import { cn, stripMarkdown } from "@/lib/utils";
import {
  Calendar,
  CheckCircle,
  Check,
  Clock,
  FileText,
  ListChecks,
  Play,
  RefreshCw,
  Users,
  AlertTriangle,
  Focus,
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

const prepStatusConfig: Record<PrepStatus, { label: string; icon: typeof CheckCircle; color: string }> = {
  prep_needed: { label: "Prep needed", icon: FileText, color: "text-destructive" },
  agenda_needed: { label: "Agenda needed", icon: Calendar, color: "text-primary" },
  bring_updates: { label: "Bring updates", icon: Clock, color: "text-primary" },
  context_needed: { label: "Context needed", icon: Users, color: "text-muted-foreground" },
  prep_ready: { label: "Prep ready", icon: CheckCircle, color: "text-success" },
  draft_ready: { label: "Draft ready", icon: FileText, color: "text-success" },
  done: { label: "Done", icon: CheckCircle, color: "text-success" },
};

const severityStyles: Record<AlertSeverity, string> = {
  critical: "border-l-destructive bg-destructive/5",
  warning: "border-l-primary bg-primary/5",
  info: "border-l-muted-foreground",
};

type WorkflowPhase = "preparing" | "enriching" | "delivering";

const phaseSteps: { key: WorkflowPhase; label: string; icon: typeof Database }[] = [
  { key: "preparing", label: "Prepare", icon: Database },
  { key: "enriching", label: "Enrich", icon: Wand2 },
  { key: "delivering", label: "Deliver", icon: Package },
];

// I4: Motivational quotes as personality layer
// Shuffled on each workflow run to keep it fresh
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

  // On mount: load data + check if workflow is already running (walk-away support)
  useEffect(() => {
    loadWeek();

    // Check if the week workflow is already in progress (user navigated away and back)
    invoke<{
      status: string;
      phase?: WorkflowPhase;
    }>("get_workflow_status", { workflow: "week" })
      .then((status) => {
        if (status.status === "running") {
          setRunning(true);
          setPhase(status.phase ?? "preparing");
        }
      })
      .catch(() => {});
  }, [loadWeek]);

  // Poll workflow status while running
  useEffect(() => {
    if (!running) return;

    // Track whether we've seen the workflow actually start.
    // The first polls may return stale status before the executor picks up the message.
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
        // If !sawRunning, any status is stale from a previous run — keep polling
      } catch {
        // Ignore polling errors
      }
    }, 1000);

    // Safety timeout: if nothing happens after 5 minutes, stop polling
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
      setError(err instanceof Error ? err.message : "Failed to queue week workflow");
    }
  }, []);

  const handleOpenWizard = useCallback(() => {
    emit("show-week-wizard");
  }, []);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-64" />
          <Skeleton className="h-4 w-48" />
        </div>
        <div className="grid grid-cols-5 gap-4">
          {[1, 2, 3, 4, 5].map((i) => (
            <Skeleton key={i} className="h-96" />
          ))}
        </div>
      </main>
    );
  }

  if (!data) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="flex h-full flex-col items-center justify-center text-center">
          {running ? (
            <WorkflowProgress phase={phase ?? "preparing"} />
          ) : (
            <>
              <Calendar className="mb-4 size-12 text-muted-foreground/30" />
              <p className="text-lg font-medium">No week overview yet</p>
              <p className="mt-1 max-w-sm text-sm text-muted-foreground">
                Run the week workflow to generate your weekly overview with meetings, actions, and focus blocks.
              </p>
              <Button
                className="mt-4 gap-1.5"
                onClick={handleRunWeek}
              >
                <Play className="size-3.5" />
                Run /week
              </Button>
            </>
          )}
          {error && (
            <Card className="mt-6 max-w-md border-destructive text-left">
              <CardContent className="pt-4">
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
              </CardContent>
            </Card>
          )}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Header */}
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">
                Week {data.weekNumber}
              </h1>
              <p className="text-sm text-muted-foreground">{data.dateRange}</p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5"
                onClick={handleOpenWizard}
              >
                <ListChecks className="size-3.5" />
                Plan this week
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="gap-1.5"
                onClick={handleRunWeek}
                disabled={running}
              >
                {running ? (
                  <RefreshCw className="size-3.5 animate-spin" />
                ) : (
                  <RefreshCw className="size-3.5" />
                )}
                {running
                  ? phase
                    ? phaseSteps.find((s) => s.key === phase)?.label ?? "Running..."
                    : "Running..."
                  : "Refresh"}
              </Button>
            </div>
          </div>

          {/* Week calendar grid */}
          <div className="mb-8 grid grid-cols-5 gap-3">
            {data.days.map((day) => (
              <DayColumn
                key={day.dayName}
                day={day}
                timeBlocks={data.availableTimeBlocks?.filter(
                  (b) => b.day === day.dayName
                )}
              />
            ))}
          </div>

          {/* Side panels */}
          <div className="grid gap-6 lg:grid-cols-2">
            {/* Action summary */}
            {data.actionSummary && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <AlertTriangle className="size-4" />
                    Action Summary
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Overdue</span>
                    <Badge variant="destructive">{data.actionSummary.overdueCount}</Badge>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Due this week</span>
                    <Badge variant="secondary">{data.actionSummary.dueThisWeek}</Badge>
                  </div>
                  {data.actionSummary.criticalItems.length > 0 && (
                    <div className="pt-2">
                      <p className="mb-2 text-sm font-medium text-destructive">
                        Critical Items:
                      </p>
                      <ul className="space-y-1">
                        {data.actionSummary.criticalItems.map((item, i) => (
                          <li key={i} className="text-sm text-muted-foreground">
                            • {stripMarkdown(item)}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </CardContent>
              </Card>
            )}

            {/* Hygiene alerts */}
            {data.hygieneAlerts && data.hygieneAlerts.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <AlertTriangle className="size-4 text-destructive" />
                    Hygiene Alerts
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  {data.hygieneAlerts.map((alert, i) => (
                    <div
                      key={i}
                      className={cn(
                        "rounded-md border-l-4 p-3",
                        severityStyles[alert.severity]
                      )}
                    >
                      <p className="font-medium">{alert.account}</p>
                      {(alert.lifecycle || alert.arr) && (
                        <p className="text-xs text-muted-foreground capitalize">
                          {alert.lifecycle && alert.lifecycle}
                          {alert.lifecycle && alert.arr && " • "}
                          {alert.arr && `ARR: ${alert.arr}`}
                        </p>
                      )}
                      <p className="mt-1 text-sm text-muted-foreground">
                        {alert.issue}
                      </p>
                    </div>
                  ))}
                </CardContent>
              </Card>
            )}
          </div>

          {/* Focus areas */}
          {data.focusAreas && data.focusAreas.length > 0 && (
            <Card className="mt-6">
              <CardHeader>
                <CardTitle className="text-base">Weekly Priorities</CardTitle>
              </CardHeader>
              <CardContent>
                <ol className="list-decimal list-inside space-y-2">
                  {data.focusAreas.map((area, i) => (
                    <li key={i} className="text-sm">
                      {area}
                    </li>
                  ))}
                </ol>
              </CardContent>
            </Card>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function DayColumn({
  day,
  timeBlocks,
}: {
  day: WeekDay;
  timeBlocks?: TimeBlock[];
}) {
  const today = new Date().toLocaleDateString("en-US", { weekday: "short" });
  const isToday = day.dayName.toLowerCase().startsWith(today.toLowerCase());

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3",
        isToday && "ring-2 ring-primary"
      )}
    >
      <h3
        className={cn(
          "mb-3 text-sm font-semibold",
          isToday && "text-primary"
        )}
      >
        {day.dayName}
      </h3>
      <div className="space-y-2">
        {day.meetings.length === 0 ? (
          <p className="py-4 text-center text-xs text-muted-foreground">
            No meetings
          </p>
        ) : (
          day.meetings.map((meeting, i) => (
            <WeekMeetingCard key={i} meeting={meeting} />
          ))
        )}
      </div>
      {/* Focus time blocks */}
      {timeBlocks && timeBlocks.length > 0 && (
        <div className="mt-3 space-y-1.5 border-t pt-3">
          {timeBlocks.map((block, i) => (
            <div
              key={i}
              className="flex items-center gap-1.5 rounded-md bg-muted/50 px-2 py-1.5 text-xs text-muted-foreground"
            >
              <Focus className="size-3 shrink-0" />
              <span className="truncate">
                {block.suggestedUse || "Focus"}{" "}
                <span className="font-mono text-[0.65rem] opacity-60">
                  {block.start}–{block.end}
                </span>
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const defaultPrepStatus = { label: "Unknown", icon: Clock, color: "text-muted-foreground" } as const;

function WeekMeetingCard({ meeting }: { meeting: WeekMeeting }) {
  const config = prepStatusConfig[meeting.prepStatus] ?? defaultPrepStatus;
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "rounded-md border p-2 text-xs",
        meeting.type === "customer" && "border-l-2 border-l-primary"
      )}
    >
      <div className="mb-1 font-mono text-muted-foreground">{meeting.time}</div>
      <div className="line-clamp-2 font-medium">{meeting.title}</div>
      <div className={cn("mt-1 flex items-center gap-1", config.color)}>
        <Icon className="size-3" />
        <span>{config.label}</span>
      </div>
    </div>
  );
}

function WorkflowProgress({ phase }: { phase: WorkflowPhase }) {
  const [messageIndex, setMessageIndex] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const startTime = useRef(Date.now());
  // Shuffle messages once on mount so repeat runs feel different
  const messages = useRef(
    [...waitingMessages].sort(() => Math.random() - 0.5)
  );

  const currentStepIndex = phaseSteps.findIndex((s) => s.key === phase);

  // Rotate messages every 6 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      setMessageIndex((i) => (i + 1) % messages.current.length);
    }, 6000);
    return () => clearInterval(interval);
  }, []);

  // Elapsed time counter
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
    <div className="flex flex-col items-center gap-6">
      <Sparkles className="size-10 text-primary animate-pulse" />

      {/* Phase steps */}
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
                    isComplete && "border-primary bg-primary text-primary-foreground",
                    isCurrent && "border-primary bg-primary/10 text-primary",
                    !isComplete && !isCurrent && "border-border text-muted-foreground"
                  )}
                >
                  {isComplete ? (
                    <Check className="size-4" />
                  ) : (
                    <StepIcon className={cn("size-4", isCurrent && "animate-pulse")} />
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

      {/* Rotating message */}
      <p className="max-w-md text-sm italic text-muted-foreground">
        {messages.current[messageIndex]}
      </p>

      {/* Elapsed time + reassurance */}
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
