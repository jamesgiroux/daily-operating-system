import { useState, useEffect, useCallback } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError } from "@/components/PageState";
import { cn } from "@/lib/utils";
import {
  ArrowLeft,
  Calendar,
  CheckCircle2,
  Circle,
  Building2,
  MessageSquare,
  Clock,
} from "lucide-react";
import type { ActionDetail } from "@/types";

const priorityStyles: Record<string, string> = {
  high: "border-destructive/50 text-destructive",
  medium: "border-yellow-500/50 text-yellow-600",
  low: "border-muted-foreground/50 text-muted-foreground",
};

export default function ActionDetailPage() {
  const { actionId } = useParams({ strict: false }) as {
    actionId?: string;
  };
  const [detail, setDetail] = useState<ActionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState(false);

  const load = useCallback(async () => {
    if (!actionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ActionDetail>("get_action_detail", {
        actionId,
      });
      setDetail(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [actionId]);

  useEffect(() => {
    load();
  }, [load]);

  async function toggleStatus() {
    if (!detail) return;
    setToggling(true);
    try {
      if (detail.status === "completed") {
        await invoke("reopen_action", { id: detail.id });
      } else {
        await invoke("complete_action", { id: detail.id });
      }
      await load();
    } finally {
      setToggling(false);
    }
  }

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-96" />
        <Skeleton className="mt-6 h-48 w-full" />
      </main>
    );
  }

  if (error || !detail) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error ?? "Action not found"} onRetry={load} />
      </main>
    );
  }

  const isCompleted = detail.status === "completed";
  const hasSource = detail.sourceId && detail.sourceMeetingTitle;

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="mx-auto max-w-2xl space-y-6 p-6">
          {/* Back link */}
          <button
            onClick={() => window.history.back()}
            className="inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            Back
          </button>

          {/* Header */}
          <div className="space-y-3">
            <div className="flex items-start gap-3">
              <button
                onClick={toggleStatus}
                disabled={toggling}
                className="mt-1 shrink-0 transition-colors hover:text-primary disabled:opacity-50"
                title={isCompleted ? "Reopen action" : "Complete action"}
              >
                {isCompleted ? (
                  <CheckCircle2 className="size-5 text-primary" />
                ) : (
                  <Circle className="size-5 text-muted-foreground" />
                )}
              </button>
              <h1
                className={cn(
                  "text-xl font-semibold tracking-tight",
                  isCompleted && "line-through opacity-60"
                )}
              >
                {detail.title}
              </h1>
            </div>

            <div className="flex flex-wrap items-center gap-2">
              <Badge
                variant="outline"
                className={cn("text-xs", priorityStyles[detail.priority])}
              >
                {detail.priority}
              </Badge>
              <Badge
                variant="outline"
                className={cn(
                  "text-xs",
                  isCompleted
                    ? "border-primary/50 text-primary"
                    : "text-muted-foreground"
                )}
              >
                {isCompleted ? "Completed" : "Open"}
              </Badge>
              {detail.waitingOn && (
                <Badge variant="outline" className="text-xs">
                  Waiting on: {detail.waitingOn}
                </Badge>
              )}
            </div>
          </div>

          {/* Context — the AI-generated reasoning */}
          {detail.context && (
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="flex items-center gap-2 text-sm font-medium">
                  <MessageSquare className="size-4 text-muted-foreground" />
                  Context
                </CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm leading-relaxed whitespace-pre-line">
                  {detail.context}
                </p>
              </CardContent>
            </Card>
          )}

          {/* Source & Account */}
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium">Details</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {/* Source meeting */}
              {hasSource && (
                <div className="flex items-start gap-3 text-sm">
                  <Calendar className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                  <div>
                    <span className="text-muted-foreground">
                      From meeting:{" "}
                    </span>
                    <Link
                      to="/meeting/history/$meetingId"
                      params={{ meetingId: detail.sourceId! }}
                      className="text-primary transition-colors hover:underline"
                    >
                      {detail.sourceMeetingTitle}
                    </Link>
                  </div>
                </div>
              )}

              {/* Source type (when no meeting link) */}
              {detail.sourceType && !hasSource && (
                <div className="flex items-start gap-3 text-sm">
                  <Calendar className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                  <div>
                    <span className="text-muted-foreground">Source: </span>
                    <span className="capitalize">{detail.sourceType}</span>
                    {detail.sourceLabel && (
                      <span className="text-muted-foreground">
                        {" "}
                        — {detail.sourceLabel}
                      </span>
                    )}
                  </div>
                </div>
              )}

              {/* Account */}
              {detail.accountName && detail.accountId && (
                <div className="flex items-start gap-3 text-sm">
                  <Building2 className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                  <div>
                    <span className="text-muted-foreground">Account: </span>
                    <Link
                      to="/accounts/$accountId"
                      params={{ accountId: detail.accountId }}
                      className="text-primary transition-colors hover:underline"
                    >
                      {detail.accountName}
                    </Link>
                  </div>
                </div>
              )}

              {/* Due date */}
              {detail.dueDate && (
                <div className="flex items-start gap-3 text-sm">
                  <Clock className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                  <div>
                    <span className="text-muted-foreground">Due: </span>
                    <span>{formatDate(detail.dueDate)}</span>
                  </div>
                </div>
              )}

              {/* Created */}
              <div className="flex items-start gap-3 text-sm">
                <Clock className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                <div>
                  <span className="text-muted-foreground">Created: </span>
                  <span>{formatDate(detail.createdAt)}</span>
                </div>
              </div>

              {/* Completed */}
              {detail.completedAt && (
                <div className="flex items-start gap-3 text-sm">
                  <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-primary" />
                  <div>
                    <span className="text-muted-foreground">Completed: </span>
                    <span>{formatDate(detail.completedAt)}</span>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Toggle action */}
          <div className="flex justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={toggleStatus}
              disabled={toggling}
            >
              {isCompleted ? "Reopen" : "Mark Complete"}
            </Button>
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function formatDate(dateStr: string): string {
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
