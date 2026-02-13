import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Link } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type {
  FocusData,
  FocusMeeting,
  PrioritizedFocusAction,
} from "@/types";
import { buildFocusViewModel } from "./focusViewModel";
import { PageEmpty, PageError } from "@/components/PageState";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { cn, stripMarkdown } from "@/lib/utils";
import { toast } from "sonner";
import {
  AlertCircle,
  ArrowLeft,
  Clock,
  Circle,
  RefreshCw,
  ShieldAlert,
  Target,
} from "lucide-react";

interface FocusResult {
  status: "success" | "not_found" | "error";
  data?: FocusData;
  message?: string;
}

const meetingTypeLabels: Record<string, string> = {
  customer: "Customer",
  qbr: "QBR",
  partnership: "Partner",
  external: "External",
  one_on_one: "1:1",
};

const priorityStyles: Record<string, string> = {
  P1: "bg-destructive/15 text-destructive",
  P2: "bg-primary/15 text-primary",
  P3: "bg-muted text-muted-foreground",
};

function formatTimeFromIso(iso: string): string {
  const match = iso.match(/T(\d{2}):(\d{2})/);
  if (!match) return iso;
  const hour = parseInt(match[1], 10);
  const minute = match[2];
  const period = hour >= 12 ? "PM" : "AM";
  const displayHour = hour === 0 ? 12 : hour > 12 ? hour - 12 : hour;
  return `${displayHour}:${minute} ${period}`;
}

function formatDuration(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const remaining = minutes % 60;
  return remaining > 0 ? `${hours}h ${remaining}m` : `${hours}h`;
}

export default function FocusPage() {
  const { personality } = usePersonality();
  const [data, setData] = useState<FocusData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const loadFocus = useCallback(async (showLoading = false) => {
    if (showLoading) {
      setLoading(true);
    }
    setError(null);

    try {
      const result = await invoke<FocusResult>("get_focus_data");
      if (result.status === "success" && result.data) {
        setData(result.data);
      } else if (result.status === "not_found") {
        setData(null);
      } else if (result.status === "error") {
        setError(result.message || "Failed to load focus data");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      if (showLoading) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    loadFocus(true);
  }, [loadFocus]);

  useEffect(() => {
    let unlistenWorkflow: UnlistenFn | undefined;
    let unlistenDelivered: UnlistenFn | undefined;
    let cancelled = false;

    listen<string>("workflow-completed", () => {
      void loadFocus(false);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenWorkflow = fn;
      }
    });

    listen<string>("operation-delivered", (event) => {
      if (event.payload === "briefing") {
        void loadFocus(false);
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenDelivered = fn;
      }
    });

    return () => {
      cancelled = true;
      unlistenWorkflow?.();
      unlistenDelivered?.();
    };
  }, [loadFocus]);

  const refreshFocus = useCallback(async () => {
    setRefreshing(true);
    try {
      await invoke<string>("refresh_focus");
      await loadFocus(false);
      toast.success("Focus refreshed");
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unknown error";
      toast.error(`Focus refresh failed: ${message}`);
      await loadFocus(false);
    } finally {
      setRefreshing(false);
    }
  }, [loadFocus]);

  const viewModel = useMemo(() => (data ? buildFocusViewModel(data) : null), [data]);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} />
      </main>
    );
  }

  if (!data) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageEmpty
          icon={Target}
          {...getPersonalityCopy("focus-empty", personality)}
        />
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="mx-auto max-w-2xl p-6">
          <div className="mb-6">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-3">
                <Link
                  to="/"
                  className="text-muted-foreground transition-colors hover:text-foreground"
                >
                  <ArrowLeft className="size-5" />
                </Link>
                <h1 className="text-2xl font-semibold tracking-tight">Today's Focus</h1>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={refreshFocus}
                disabled={refreshing}
                className="gap-2"
              >
                <RefreshCw className={cn("size-3.5", refreshing && "animate-spin")} />
                Refresh Focus
              </Button>
            </div>
          </div>

          {data.focusStatement && (
            <div className="mb-6 rounded-lg border border-success/15 bg-success/10 p-5">
              <p className="text-lg font-medium leading-relaxed">{data.focusStatement}</p>
            </div>
          )}

          {data.availability.warnings.length > 0 && (
            <section className="mb-6 rounded-lg border border-amber-500/40 bg-amber-500/10 p-4">
              <div className="mb-1 flex items-center gap-2 text-sm font-semibold">
                <AlertCircle className="size-4" />
                Capacity is in degraded mode
              </div>
              {data.availability.warnings.map((warning, idx) => (
                <p key={idx} className="text-sm text-muted-foreground">
                  {warning}
                </p>
              ))}
            </section>
          )}

          <CapacitySection data={data} />

          <TopThreeSection
            actions={viewModel?.topThree ?? []}
            summary={data.implications.summary}
          />

          {(viewModel?.atRisk.length ?? 0) > 0 && (
            <AtRiskSection actions={viewModel?.atRisk ?? []} />
          )}

          <OtherPrioritiesSection
            actions={viewModel?.otherPrioritiesVisible ?? []}
            total={viewModel?.otherPrioritiesP1Total ?? 0}
            showViewAll={viewModel?.showViewAllActions ?? false}
          />

          {data.keyMeetings.length > 0 && <KeyMeetingsSection meetings={data.keyMeetings} />}

          {data.availableBlocks.length > 0 && (
            <AvailableTimeSection
              blocks={data.availableBlocks}
              totalMinutes={data.totalFocusMinutes}
            />
          )}

          <div className="h-12" />
        </div>
      </ScrollArea>
    </main>
  );
}

function CapacitySection({ data }: { data: FocusData }) {
  return (
    <section className="mb-6">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Clock className="size-4" />
            Capacity Snapshot
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-2 gap-3 text-sm">
            <Metric label="Meetings" value={`${data.availability.meetingCount}`} />
            <Metric
              label="In meetings"
              value={formatDuration(data.availability.meetingMinutes)}
            />
            <Metric
              label="Available"
              value={formatDuration(data.availability.availableMinutes)}
            />
            <Metric
              label="Deep work"
              value={formatDuration(data.availability.deepWorkMinutes)}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            Source: {data.availability.source === "live" ? "Live calendar" : "Briefing fallback"}
          </p>
        </CardContent>
      </Card>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md bg-muted/40 p-2.5">
      <div className="text-muted-foreground">{label}</div>
      <div className="font-medium text-foreground">{value}</div>
    </div>
  );
}

function TopThreeSection({
  actions,
  summary,
}: {
  actions: PrioritizedFocusAction[];
  summary: string;
}) {
  return (
    <section className="mb-6">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Target className="size-4" />
            Top 3 If You Do Nothing Else
          </CardTitle>
        </CardHeader>
        <CardContent>
          {actions.length === 0 ? (
            <p className="text-sm text-muted-foreground">No actions to prioritize.</p>
          ) : (
            <div className="space-y-1">
              {actions.map((action) => (
                <PrioritizedActionRow key={action.action.id} item={action} showReason />
              ))}
            </div>
          )}
          <p className="mt-3 text-xs text-muted-foreground">{summary}</p>
        </CardContent>
      </Card>
    </section>
  );
}

function AtRiskSection({ actions }: { actions: PrioritizedFocusAction[] }) {
  return (
    <section className="mb-6">
      <Card className="border-destructive/25">
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base text-destructive">
            <ShieldAlert className="size-4" />
            At Risk Today
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-1">
          {actions.map((action) => (
            <PrioritizedActionRow key={action.action.id} item={action} showReason />
          ))}
        </CardContent>
      </Card>
    </section>
  );
}

function OtherPrioritiesSection({
  actions,
  total,
  showViewAll,
}: {
  actions: PrioritizedFocusAction[];
  total: number;
  showViewAll: boolean;
}) {
  if (actions.length === 0) return null;

  return (
    <section className="mb-6">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Target className="size-4" />
            Other Priorities
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-1">
          {actions.map((action) => (
            <PrioritizedActionRow key={action.action.id} item={action} />
          ))}
          {showViewAll && (
            <Link
              to="/actions"
              search={{ search: undefined }}
              className="mt-2 block text-center text-sm text-primary hover:underline"
            >
              View All Actions ({total})
            </Link>
          )}
        </CardContent>
      </Card>
    </section>
  );
}

function PrioritizedActionRow({
  item,
  showReason = false,
}: {
  item: PrioritizedFocusAction;
  showReason?: boolean;
}) {
  const action = item.action;
  const isOverdue =
    action.dueDate && new Date(action.dueDate) < new Date(new Date().toDateString());
  const style = priorityStyles[action.priority] || priorityStyles.P3;

  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      className="block rounded-md p-2.5 transition-colors hover:bg-muted/50"
    >
      <div className="flex items-start gap-3">
        <Circle className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-2">
            <span className="text-sm font-medium">{stripMarkdown(action.title)}</span>
            <div className="flex items-center gap-2">
              <Badge className={cn("shrink-0 text-xs", style)} variant="secondary">
                {action.priority}
              </Badge>
              <span className="text-xs text-muted-foreground">
                {formatDuration(item.effortMinutes)}
              </span>
              {!item.feasible && (
                <Badge variant="outline" className="text-xs">
                  Stretch
                </Badge>
              )}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-xs">
            {action.accountId && <span className="text-primary">{action.accountId}</span>}
            {action.dueDate && (
              <span
                className={cn(
                  "flex items-center gap-1",
                  isOverdue ? "text-destructive" : "text-muted-foreground",
                )}
              >
                {isOverdue && <AlertCircle className="size-3" />}
                {action.dueDate}
              </span>
            )}
            <span className="text-muted-foreground">Score {item.score}</span>
          </div>
          {showReason && (
            <p className="mt-1 text-xs text-muted-foreground">{item.reason}</p>
          )}
        </div>
      </div>
    </Link>
  );
}

function KeyMeetingsSection({ meetings }: { meetings: FocusMeeting[] }) {
  return (
    <section className="mb-6">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Clock className="size-4" />
            Key Meetings
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            {meetings.map((m) => (
              <MeetingRow key={m.id} meeting={m} />
            ))}
          </div>
        </CardContent>
      </Card>
    </section>
  );
}

function MeetingRow({ meeting }: { meeting: FocusMeeting }) {
  const label = meetingTypeLabels[meeting.meetingType] || meeting.meetingType;

  return (
    <div className="flex items-center gap-3 rounded-md p-2.5 transition-colors hover:bg-muted/50">
      <span className="w-20 shrink-0 font-mono text-sm text-muted-foreground">
        {formatTimeFromIso(meeting.time)}
      </span>
      <div className="min-w-0 flex-1">
        <span className="text-sm font-medium">{meeting.title}</span>
        {meeting.account && <span className="ml-2 text-xs text-primary">{meeting.account}</span>}
      </div>
      <div className="flex items-center gap-2">
        <Badge variant="outline" className="text-xs">
          {label}
        </Badge>
        {meeting.hasPrep && (
          <Badge variant="secondary" className="bg-success/15 text-xs text-success">
            Prepped
          </Badge>
        )}
      </div>
    </div>
  );
}

function AvailableTimeSection({
  blocks,
  totalMinutes,
}: {
  blocks: FocusData["availableBlocks"];
  totalMinutes: number;
}) {
  return (
    <section className="mb-6">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-sm font-semibold">
          <Clock className="size-4 text-muted-foreground" />
          Available Time Blocks
        </h2>
        <span className="text-sm text-muted-foreground">{formatDuration(totalMinutes)} total</span>
      </div>
      <div className="space-y-2">
        {blocks.map((block, i) => (
          <div
            key={i}
            className="flex items-center justify-between rounded-md bg-muted/50 px-3 py-2"
          >
            <div>
              <span className="font-mono text-sm">
                {formatTimeFromIso(block.start)} - {formatTimeFromIso(block.end)}
              </span>
              {block.suggestedUse && (
                <span className="ml-3 text-xs text-muted-foreground">{block.suggestedUse}</span>
              )}
            </div>
            <span className="text-xs text-muted-foreground">{formatDuration(block.durationMinutes)}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
