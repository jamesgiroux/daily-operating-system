import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { FocusData, FocusMeeting, DbAction } from "@/types";
import { PageEmpty, PageError } from "@/components/PageState";
import { cn, stripMarkdown } from "@/lib/utils";
import {
  ArrowLeft,
  Target,
  Clock,
  AlertCircle,
  Circle,
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
  const [data, setData] = useState<FocusData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadFocus() {
      try {
        const result = await invoke<FocusResult>("get_focus_data");
        if (result.status === "success" && result.data) {
          setData(result.data);
        } else if (result.status === "not_found") {
          // No briefing run yet — show empty state (data stays null)
        } else if (result.status === "error") {
          setError(result.message || "Failed to load focus data");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadFocus();
  }, []);

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
          title="No focus data yet"
          message="Your focus priorities will appear here after the daily briefing runs."
        />
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="mx-auto max-w-2xl p-6">
          {/* Header */}
          <div className="mb-6">
            <div className="flex items-center gap-3">
              <Link
                to="/"
                className="text-muted-foreground hover:text-foreground transition-colors"
              >
                <ArrowLeft className="size-5" />
              </Link>
              <h1 className="text-2xl font-semibold tracking-tight">
                Today's Focus
              </h1>
            </div>
          </div>

          {/* Focus hero */}
          {data.focusStatement && (
            <div className="mb-8 rounded-lg border border-success/15 bg-success/10 p-5">
              <p className="text-lg font-medium leading-relaxed">
                {data.focusStatement}
              </p>
            </div>
          )}

          {/* Priorities */}
          <PrioritiesSection priorities={data.priorities} />

          {/* Key meetings */}
          {data.keyMeetings.length > 0 && (
            <KeyMeetingsSection meetings={data.keyMeetings} />
          )}

          {/* Available time */}
          {data.availableBlocks.length > 0 && (
            <AvailableTimeSection
              blocks={data.availableBlocks}
              totalMinutes={data.totalFocusMinutes}
            />
          )}

          {/* Explicit end — breathing room */}
          <div className="h-12" />
        </div>
      </ScrollArea>
    </main>
  );
}

function PrioritiesSection({ priorities }: { priorities: DbAction[] }) {
  const MAX_VISIBLE = 8;
  const visible = priorities.slice(0, MAX_VISIBLE);
  const hasMore = priorities.length > MAX_VISIBLE;

  return (
    <section className="mb-6">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Target className="size-4" />
            Today's Priorities
          </CardTitle>
        </CardHeader>
        <CardContent>
          {visible.length === 0 ? (
            <p className="py-4 text-center text-sm text-muted-foreground">
              No actions due today.
            </p>
          ) : (
            <div className="space-y-1">
              {visible.map((action) => (
                <PriorityRow key={action.id} action={action} />
              ))}
              {hasMore && (
                <Link
                  to="/actions"
                  search={{ search: undefined }}
                  className="mt-2 block text-center text-sm text-primary hover:underline"
                >
                  View all {priorities.length} actions
                </Link>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}

function PriorityRow({ action }: { action: DbAction }) {
  const isOverdue =
    action.dueDate && new Date(action.dueDate) < new Date(new Date().toDateString());
  const style = priorityStyles[action.priority] || priorityStyles.P3;

  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      className="flex items-start gap-3 rounded-md p-2.5 transition-colors hover:bg-muted/50"
    >
      <Circle className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="flex items-start justify-between gap-2">
          <span className="text-sm font-medium">
            {stripMarkdown(action.title)}
          </span>
          <Badge
            className={cn("shrink-0 text-xs", style)}
            variant="secondary"
          >
            {action.priority}
          </Badge>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-xs">
          {action.accountId && (
            <span className="text-primary">{action.accountId}</span>
          )}
          {action.dueDate && (
            <span
              className={cn(
                "flex items-center gap-1",
                isOverdue ? "text-destructive" : "text-muted-foreground"
              )}
            >
              {isOverdue && <AlertCircle className="size-3" />}
              {action.dueDate}
            </span>
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
  const label =
    meetingTypeLabels[meeting.meetingType] || meeting.meetingType;

  return (
    <div className="flex items-center gap-3 rounded-md p-2.5 transition-colors hover:bg-muted/50">
      <span className="w-20 shrink-0 font-mono text-sm text-muted-foreground">
        {formatTimeFromIso(meeting.time)}
      </span>
      <div className="min-w-0 flex-1">
        <span className="text-sm font-medium">{meeting.title}</span>
        {meeting.account && (
          <span className="ml-2 text-xs text-primary">
            {meeting.account}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Badge variant="outline" className="text-xs">
          {label}
        </Badge>
        {meeting.hasPrep && (
          <Badge
            variant="secondary"
            className="bg-success/15 text-success text-xs"
          >
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
          Available Time
        </h2>
        <span className="text-sm text-muted-foreground">
          {formatDuration(totalMinutes)} total
        </span>
      </div>
      <div className="space-y-2">
        {blocks.map((block, i) => (
          <div
            key={i}
            className="flex items-center justify-between rounded-md bg-muted/50 px-3 py-2"
          >
            <div>
              <span className="font-mono text-sm">
                {formatTimeFromIso(block.start)} –{" "}
                {formatTimeFromIso(block.end)}
              </span>
              {block.suggestedUse && (
                <span className="ml-3 text-xs text-muted-foreground">
                  {block.suggestedUse}
                </span>
              )}
            </div>
            <span className="text-xs text-muted-foreground">
              {formatDuration(block.durationMinutes)}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
