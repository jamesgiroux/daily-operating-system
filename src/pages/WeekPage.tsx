import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

import type {
  WeekOverview,
  WeekMeeting,
  DayShape,
  ReadinessCheck,
  WeekAction,
  AlertSeverity,
  PrepStatus,
} from "@/types";
import { cn, stripMarkdown } from "@/lib/utils";
import {
  Calendar,
  CheckCircle,
  Check,
  ChevronDown,
  Clock,
  FileText,
  Play,
  RefreshCw,
  Users,
  AlertTriangle,
  Sparkles,
  Database,
  Wand2,
  Package,
  ShieldAlert,
  CircleAlert,
  Info,
} from "lucide-react";

interface WeekResult {
  status: "success" | "not_found" | "error";
  data?: WeekOverview;
  message?: string;
}

const prepStatusConfig: Record<
  PrepStatus,
  { label: string; icon: typeof CheckCircle; color: string }
> = {
  prep_needed: {
    label: "Prep needed",
    icon: FileText,
    color: "text-destructive",
  },
  agenda_needed: {
    label: "Agenda needed",
    icon: Calendar,
    color: "text-primary",
  },
  bring_updates: {
    label: "Bring updates",
    icon: Clock,
    color: "text-primary",
  },
  context_needed: {
    label: "Context needed",
    icon: Users,
    color: "text-muted-foreground",
  },
  prep_ready: {
    label: "Prep ready",
    icon: CheckCircle,
    color: "text-success",
  },
  draft_ready: {
    label: "Draft ready",
    icon: FileText,
    color: "text-success",
  },
  done: { label: "Done", icon: CheckCircle, color: "text-success" },
};

const severityStyles: Record<AlertSeverity, string> = {
  critical: "border-l-destructive bg-destructive/5",
  warning: "border-l-primary bg-primary/5",
  info: "border-l-muted-foreground",
};

const densityConfig: Record<string, { label: string; color: string; barColor: string }> = {
  light: { label: "Light", color: "text-success", barColor: "bg-success/60" },
  moderate: { label: "Moderate", color: "text-primary", barColor: "bg-primary/60" },
  busy: { label: "Busy", color: "text-amber-600", barColor: "bg-amber-500/60" },
  packed: { label: "Packed", color: "text-destructive", barColor: "bg-destructive/60" },
};

type WorkflowPhase = "preparing" | "enriching" | "delivering";

const phaseSteps: { key: WorkflowPhase; label: string; icon: typeof Database }[] = [
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

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="p-8">
          <div className="mx-auto max-w-6xl space-y-8">
            <div className="space-y-2">
              <Skeleton className="h-8 w-64" />
              <Skeleton className="h-4 w-48" />
            </div>
            <Skeleton className="h-24" />
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <Skeleton key={i} className="h-16" />
              ))}
            </div>
          </div>
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
                Run the week workflow to generate your weekly briefing with
                readiness checks, day shapes, and actions.
              </p>
              <Button className="mt-4 gap-1.5" onClick={handleRunWeek}>
                <Play className="size-3.5" />
                Run /week
              </Button>
            </>
          )}
          {error && <ErrorCard error={error} />}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-8">
          <div className="mx-auto max-w-6xl space-y-8">
            {/* Header */}
            <WeekHeader
              data={data}
              running={running}
              phase={phase}
              onRunWeek={handleRunWeek}
            />

            {/* Running state overlay */}
            {running && <WorkflowProgress phase={phase ?? "preparing"} />}

            {/* Readiness checks */}
            {data.readinessChecks && data.readinessChecks.length > 0 && (
              <ReadinessSection checks={data.readinessChecks} />
            )}

            {/* Week shape */}
            {data.dayShapes && data.dayShapes.length > 0 && (
              <WeekShapeSection shapes={data.dayShapes} />
            )}

            {/* Actions */}
            {data.actionSummary && (
              <ActionsSection summary={data.actionSummary} />
            )}

            {/* Hygiene alerts */}
            {data.hygieneAlerts && data.hygieneAlerts.length > 0 && (
              <AccountHealthSection alerts={data.hygieneAlerts} />
            )}

            {error && <ErrorCard error={error} />}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

// ---------------------------------------------------------------------------
// Section Components
// ---------------------------------------------------------------------------

function WeekHeader({
  data,
  running,
  phase,
  onRunWeek,
}: {
  data: WeekOverview;
  running: boolean;
  phase: WorkflowPhase | null;
  onRunWeek: () => void;
}) {
  return (
    <div className="flex items-start justify-between">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">
          Week {data.weekNumber}
        </h1>
        <p className="text-sm text-muted-foreground">{data.dateRange}</p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="gap-1.5"
        onClick={onRunWeek}
        disabled={running}
      >
        {running ? (
          <RefreshCw className="size-3.5 animate-spin" />
        ) : (
          <RefreshCw className="size-3.5" />
        )}
        {running
          ? phase
            ? (phaseSteps.find((s) => s.key === phase)?.label ?? "Running...")
            : "Running..."
          : "Refresh"}
      </Button>
    </div>
  );
}

function ReadinessSection({ checks }: { checks: ReadinessCheck[] }) {
  const actionNeeded = checks.filter((c) => c.severity === "action_needed");
  const headsUp = checks.filter((c) => c.severity === "heads_up");
  const sorted = [...actionNeeded, ...headsUp];

  return (
    <Card className="border-amber-200 dark:border-amber-800">
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <ShieldAlert className="size-4 text-amber-600" />
          Readiness
          <Badge variant="outline" className="ml-auto font-mono text-xs">
            {checks.length}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {sorted.map((check, i) => (
          <div
            key={i}
            className={cn(
              "flex items-start gap-2.5 rounded-md p-2.5 text-sm",
              check.severity === "action_needed"
                ? "bg-destructive/5"
                : "bg-muted/50"
            )}
          >
            {check.severity === "action_needed" ? (
              <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-destructive" />
            ) : (
              <Info className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
            )}
            <span className="text-muted-foreground">
              {check.message}
            </span>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

function WeekShapeSection({ shapes }: { shapes: DayShape[] }) {
  const todayStr = new Date().toISOString().split("T")[0];
  const maxMinutes = Math.max(...shapes.map((s) => s.meetingMinutes), 1);

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">Week Shape</CardTitle>
      </CardHeader>
      <CardContent className="space-y-1">
        {shapes.map((shape) => {
          const isToday = shape.date === todayStr;
          const config = densityConfig[shape.density] ?? densityConfig.light;
          const barWidth = Math.max(
            (shape.meetingMinutes / maxMinutes) * 100,
            shape.meetingMinutes > 0 ? 8 : 0
          );

          const focusMinutes = shape.availableBlocks.reduce(
            (sum, b) => sum + (b.durationMinutes ?? 0),
            0
          );

          return (
            <Collapsible key={shape.dayName}>
              <CollapsibleTrigger className="w-full">
                <div
                  className={cn(
                    "flex items-center gap-3 rounded-md px-3 py-2.5 transition-colors hover:bg-muted/50",
                    isToday && "ring-1 ring-primary/30 bg-primary/5"
                  )}
                >
                  {/* Day label */}
                  <div className="w-12 shrink-0 text-left">
                    <span
                      className={cn(
                        "text-sm font-medium",
                        isToday && "text-primary"
                      )}
                    >
                      {shape.dayName.slice(0, 3)}
                    </span>
                  </div>

                  {/* Density bar */}
                  <div className="flex-1">
                    <div className="h-5 w-full rounded-sm bg-muted/30">
                      {barWidth > 0 && (
                        <div
                          className={cn("h-full rounded-sm transition-all", config.barColor)}
                          style={{ width: `${barWidth}%` }}
                        />
                      )}
                    </div>
                  </div>

                  {/* Stats */}
                  <div className="flex w-40 shrink-0 items-center gap-3 text-xs text-muted-foreground">
                    <span className="w-16 text-right">
                      {shape.meetingCount === 0
                        ? "No meetings"
                        : `${shape.meetingCount} mtg${shape.meetingCount !== 1 ? "s" : ""}`}
                    </span>
                    {focusMinutes > 0 && (
                      <span className="text-success">
                        {Math.round(focusMinutes / 60)}h focus
                      </span>
                    )}
                  </div>

                  <ChevronDown className="size-3.5 shrink-0 text-muted-foreground transition-transform [[data-state=open]_&]:rotate-180" />
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent>
                {shape.meetings.length > 0 && (
                  <div className="ml-[3.75rem] space-y-1 pb-2 pr-3">
                    {shape.meetings.map((meeting, i) => (
                      <DayMeetingRow key={i} meeting={meeting} />
                    ))}
                  </div>
                )}
              </CollapsibleContent>
            </Collapsible>
          );
        })}
      </CardContent>
    </Card>
  );
}

function DayMeetingRow({ meeting }: { meeting: WeekMeeting }) {
  const config =
    prepStatusConfig[meeting.prepStatus] ?? {
      label: "Unknown",
      icon: Clock,
      color: "text-muted-foreground",
    };
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-md border px-3 py-2 text-xs",
        meeting.type === "customer" && "border-l-2 border-l-primary"
      )}
    >
      <span className="w-16 shrink-0 font-mono text-muted-foreground">
        {meeting.time}
      </span>
      <span className="flex-1 truncate font-medium">{meeting.title}</span>
      {meeting.account && (
        <Badge variant="secondary" className="shrink-0 text-[0.65rem]">
          {meeting.account}
        </Badge>
      )}
      <div className={cn("flex items-center gap-1 shrink-0", config.color)}>
        <Icon className="size-3" />
        <span className="hidden sm:inline">{config.label}</span>
      </div>
    </div>
  );
}

function ActionsSection({
  summary,
}: {
  summary: NonNullable<WeekOverview["actionSummary"]>;
}) {
  const hasOverdueItems = summary.overdue && summary.overdue.length > 0;
  const hasDueItems =
    summary.dueThisWeekItems && summary.dueThisWeekItems.length > 0;

  if (summary.overdueCount === 0 && summary.dueThisWeek === 0) return null;

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      {/* Overdue */}
      {summary.overdueCount > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <AlertTriangle className="size-4 text-destructive" />
              Overdue
              <Badge variant="destructive" className="ml-auto">
                {summary.overdueCount}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {hasOverdueItems ? (
              <div className="space-y-2">
                {summary.overdue!.map((action) => (
                  <ActionRow key={action.id} action={action} showOverdue />
                ))}
              </div>
            ) : (
              <ul className="space-y-1">
                {summary.criticalItems.map((item, i) => (
                  <li key={i} className="text-sm text-muted-foreground">
                    {stripMarkdown(item)}
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      )}

      {/* Due this week */}
      {summary.dueThisWeek > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <Clock className="size-4" />
              Due This Week
              <Badge variant="secondary" className="ml-auto">
                {summary.dueThisWeek}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {hasDueItems ? (
              <div className="space-y-2">
                {summary.dueThisWeekItems!.map((action) => (
                  <ActionRow key={action.id} action={action} />
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">
                {summary.dueThisWeek} action
                {summary.dueThisWeek !== 1 ? "s" : ""} due this week
              </p>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function ActionRow({
  action,
  showOverdue,
}: {
  action: WeekAction;
  showOverdue?: boolean;
}) {
  return (
    <div className="flex items-start gap-2 rounded-md border p-2.5 text-sm">
      <div className="flex-1 min-w-0">
        <p className="font-medium leading-tight">{action.title}</p>
        <div className="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
          {action.account && (
            <Badge variant="secondary" className="text-[0.65rem]">
              {action.account}
            </Badge>
          )}
          {action.priority && action.priority !== "P3" && (
            <span
              className={cn(
                "font-mono",
                action.priority === "P1" && "text-destructive"
              )}
            >
              {action.priority}
            </span>
          )}
          {showOverdue && action.daysOverdue != null && action.daysOverdue > 0 && (
            <span className="text-destructive">
              {action.daysOverdue}d overdue
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

function AccountHealthSection({
  alerts,
}: {
  alerts: NonNullable<WeekOverview["hygieneAlerts"]>;
}) {
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <AlertTriangle className="size-4 text-destructive" />
          Account Health
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {alerts.map((alert, i) => (
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
                {alert.lifecycle}
                {alert.lifecycle && alert.arr && " \u2022 "}
                {alert.arr && `ARR: ${alert.arr}`}
              </p>
            )}
            <p className="mt-1 text-sm text-muted-foreground">{alert.issue}</p>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

function ErrorCard({ error }: { error: string }) {
  return (
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
