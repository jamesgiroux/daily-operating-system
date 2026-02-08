import { useState, useEffect, useCallback } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError } from "@/components/PageState";
import { cn } from "@/lib/utils";
import {
  ArrowLeft,
  Calendar,
  CheckCircle2,
  Clock,
  Users,
} from "lucide-react";
import type { MeetingHistoryDetail } from "@/types";

const meetingTypeLabels: Record<string, string> = {
  customer: "Customer",
  qbr: "QBR",
  training: "Training",
  internal: "Internal",
  team_sync: "Team Sync",
  one_on_one: "1:1",
  partnership: "Partner",
  all_hands: "All Hands",
  external: "External",
  personal: "Personal",
};

const captureStyles: Record<string, string> = {
  win: "text-green-600",
  risk: "text-destructive",
  decision: "text-primary",
};

const captureLabels: Record<string, string> = {
  win: "W",
  risk: "R",
  decision: "D",
};

const captureSectionLabels: Record<string, string> = {
  win: "Wins",
  risk: "Risks",
  decision: "Decisions",
};

export default function MeetingHistoryDetailPage() {
  const { meetingId } = useParams({ strict: false }) as { meetingId?: string };
  const [detail, setDetail] = useState<MeetingHistoryDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!meetingId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<MeetingHistoryDetail>(
        "get_meeting_history_detail",
        { meetingId }
      );
      setDetail(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-64" />
        <Skeleton className="mt-6 h-32 w-full" />
      </main>
    );
  }

  if (error || !detail) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error ?? "Meeting not found"} onRetry={load} />
      </main>
    );
  }

  // Group captures by type
  const capturesByType = detail.captures.reduce<Record<string, string[]>>(
    (acc, c) => {
      if (!acc[c.captureType]) acc[c.captureType] = [];
      acc[c.captureType].push(c.content);
      return acc;
    },
    {}
  );

  const meetingDate = formatDateTime(detail.startTime);
  const timeRange = detail.endTime
    ? `${formatTime(detail.startTime)} \u2013 ${formatTime(detail.endTime)}`
    : formatTime(detail.startTime);

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="space-y-6 p-6">
          {/* Back link */}
          <button
            onClick={() => window.history.back()}
            className="inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            Back
          </button>

          {/* Header */}
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-2xl font-semibold tracking-tight">
                {detail.title}
              </h1>
              <Badge variant="outline" className="text-xs">
                {meetingTypeLabels[detail.meetingType] ?? detail.meetingType}
              </Badge>
            </div>
            <div className="mt-1 flex items-center gap-3 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Calendar className="size-3.5" />
                {meetingDate}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="size-3.5" />
                {timeRange}
              </span>
              {detail.accountName && (
                <Link
                  to="/accounts/$accountId"
                  params={{ accountId: detail.accountId! }}
                  className="text-primary transition-colors hover:underline"
                >
                  {detail.accountName}
                </Link>
              )}
            </div>
          </div>

          {/* Summary */}
          {detail.summary && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Summary</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm leading-relaxed">{detail.summary}</p>
              </CardContent>
            </Card>
          )}

          {/* Captures by type */}
          {Object.keys(capturesByType).length > 0 && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Outcomes</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                {["win", "risk", "decision"].map(
                  (type) =>
                    capturesByType[type] &&
                    capturesByType[type].length > 0 && (
                      <div key={type}>
                        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                          {captureSectionLabels[type] ?? type}
                        </h3>
                        <div className="space-y-1.5">
                          {capturesByType[type].map((content, i) => (
                            <div
                              key={i}
                              className="flex items-start gap-2 text-sm"
                            >
                              <span
                                className={cn(
                                  "inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-bold",
                                  captureStyles[type] ?? "text-muted-foreground"
                                )}
                              >
                                {captureLabels[type] ?? "?"}
                              </span>
                              <span>{content}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )
                )}
              </CardContent>
            </Card>
          )}

          {/* Actions from this meeting */}
          {detail.actions.length > 0 && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Actions
                  <span className="ml-1 text-muted-foreground">
                    ({detail.actions.length})
                  </span>
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {detail.actions.map((a) => (
                    <div
                      key={a.id}
                      className="flex items-center gap-2 text-sm"
                    >
                      <CheckCircle2
                        className={cn(
                          "size-3.5 shrink-0",
                          a.status === "completed"
                            ? "text-primary"
                            : "text-muted-foreground"
                        )}
                      />
                      <Badge variant="outline" className="shrink-0 text-xs">
                        {a.priority}
                      </Badge>
                      <span
                        className={cn(
                          "truncate",
                          a.status === "completed" && "line-through opacity-60"
                        )}
                      >
                        {a.title}
                      </span>
                      {a.dueDate && (
                        <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                          {formatShortDate(a.dueDate)}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Attendees */}
          {detail.attendees.length > 0 && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Attendees
                  <span className="ml-1 text-muted-foreground">
                    ({detail.attendees.length})
                  </span>
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex flex-wrap gap-2">
                  {detail.attendees.map((email) => (
                    <span
                      key={email}
                      className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs"
                    >
                      <Users className="size-3 text-muted-foreground" />
                      {email}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Empty state: no outcomes at all */}
          {detail.captures.length === 0 &&
            detail.actions.length === 0 &&
            !detail.summary && (
              <Card>
                <CardContent className="flex flex-col items-center py-12 text-center">
                  <Calendar className="mb-4 size-12 text-muted-foreground/40" />
                  <p className="text-lg font-medium">
                    No outcomes recorded
                  </p>
                  <p className="text-sm text-muted-foreground">
                    Attach a transcript or capture outcomes after the meeting to
                    see data here.
                  </p>
                </CardContent>
              </Card>
            )}
        </div>
      </ScrollArea>
    </main>
  );
}

function formatDateTime(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}

function formatTime(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return dateStr;
  }
}

function formatShortDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}
