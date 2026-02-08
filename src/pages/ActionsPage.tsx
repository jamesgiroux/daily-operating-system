import { useSearch } from "@tanstack/react-router";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { useActions } from "@/hooks/useActions";
import type { DbAction } from "@/types";
import { cn, stripMarkdown } from "@/lib/utils";
import { PageError } from "@/components/PageState";
import {
  CheckCircle2,
  Circle,
  Clock,
  RefreshCw,
} from "lucide-react";

type StatusTab = "all" | "pending" | "completed" | "waiting";
type PriorityTab = "all" | "P1" | "P2" | "P3";

const statusTabs: { key: StatusTab; label: string }[] = [
  { key: "pending", label: "Pending" },
  { key: "completed", label: "Completed" },
  { key: "waiting", label: "Waiting" },
  { key: "all", label: "All" },
];

const priorityTabs: { key: PriorityTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "P1", label: "P1" },
  { key: "P2", label: "P2" },
  { key: "P3", label: "P3" },
];

const priorityStyles: Record<string, string> = {
  P1: "bg-destructive/15 text-destructive border-destructive/30",
  P2: "bg-primary/15 text-primary border-primary/30",
  P3: "bg-muted text-muted-foreground border-muted-foreground/30",
};

const priorityLabels: Record<string, string> = {
  P1: "Critical",
  P2: "High",
  P3: "Normal",
};

export default function ActionsPage() {
  const { search: initialSearch } = useSearch({ strict: false });
  const {
    actions,
    allActions,
    loading,
    error,
    refresh,
    toggleAction,
    statusFilter,
    setStatusFilter,
    priorityFilter,
    setPriorityFilter,
    searchQuery,
    setSearchQuery,
  } = useActions(initialSearch as string | undefined);

  const statusCounts: Record<StatusTab, number> = {
    all: allActions.length,
    pending: allActions.filter((a) => a.status === "pending").length,
    completed: allActions.filter((a) => a.status === "completed").length,
    waiting: allActions.filter((a) => a.status === "waiting").length,
  };

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-24 w-full" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={refresh} />
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">Actions</h1>
              <p className="text-sm text-muted-foreground">
                Track, complete, and manage action items across days
              </p>
            </div>
            <Button variant="ghost" size="icon" className="size-8" onClick={refresh}>
              <RefreshCw className="size-4" />
            </Button>
          </div>

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search actions..."
            className="mb-4"
          />

          <TabFilter
            tabs={statusTabs}
            active={statusFilter}
            onChange={setStatusFilter}
            counts={statusCounts}
            className="mb-4"
          />

          {/* Priority filter */}
          <div className="mb-6 flex gap-1.5">
            {priorityTabs.map((tab) => (
              <button
                key={tab.key}
                onClick={() => setPriorityFilter(tab.key)}
                className={cn(
                  "rounded-md px-3 py-1 text-xs font-medium transition-colors",
                  priorityFilter === tab.key
                    ? "bg-foreground/10 text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {tab.label}
              </button>
            ))}
          </div>

          {/* Actions list */}
          <div className="space-y-3">
            {actions.length === 0 ? (
              <Card>
                <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                  <CheckCircle2 className="mb-4 size-12 text-muted-foreground/40" />
                  <p className="text-lg font-medium">No actions to show</p>
                  <p className="text-sm text-muted-foreground">
                    {statusFilter === "completed"
                      ? "No completed actions yet."
                      : statusFilter === "waiting"
                        ? "Nothing waiting on others."
                        : "You're all caught up!"}
                  </p>
                </CardContent>
              </Card>
            ) : (
              actions.map((action) => (
                <ActionCard
                  key={action.id}
                  action={action}
                  onToggle={() => toggleAction(action.id)}
                />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function ActionCard({
  action,
  onToggle,
}: {
  action: DbAction;
  onToggle: () => void;
}) {
  const isOverdue =
    action.dueDate &&
    action.status === "pending" &&
    new Date(action.dueDate) < new Date();

  const isCompleted = action.status === "completed";
  const isWaiting = action.status === "waiting";

  const dueLabel = action.dueDate
    ? formatDueDate(action.dueDate)
    : null;

  return (
    <Card
      className={cn(
        "transition-all hover:-translate-y-0.5 hover:shadow-md",
        isOverdue && "border-l-4 border-l-destructive",
        isCompleted && "opacity-60"
      )}
    >
      <CardContent className="p-5">
        <div className="flex items-start gap-3">
          {/* Completion toggle */}
          <button
            onClick={onToggle}
            className={cn(
              "mt-0.5 shrink-0 transition-colors",
              isCompleted
                ? "text-primary hover:text-muted-foreground"
                : "text-muted-foreground/50 hover:text-primary"
            )}
          >
            {isCompleted ? (
              <CheckCircle2 className="size-5" />
            ) : (
              <Circle className="size-5" />
            )}
          </button>

          <div className="flex-1 space-y-1">
            <div className="flex items-center gap-2">
              <h3
                className={cn(
                  "font-medium",
                  isCompleted && "line-through"
                )}
              >
                {stripMarkdown(action.title)}
              </h3>
              <Badge
                variant="outline"
                className={cn(
                  "text-xs",
                  priorityStyles[action.priority] || priorityStyles.P2
                )}
              >
                {priorityLabels[action.priority] || action.priority}
              </Badge>
              {isWaiting && action.waitingOn && (
                <Badge variant="secondary" className="text-xs">
                  <Clock className="mr-1 size-3" />
                  Waiting on {action.waitingOn}
                </Badge>
              )}
            </div>

            {action.accountId && (
              <p className="text-sm text-primary">{action.accountId}</p>
            )}

            {action.context && (
              <p className="text-sm text-muted-foreground">{action.context}</p>
            )}

            {action.sourceLabel && (
              <p className="text-xs text-muted-foreground/70">
                Source: {action.sourceLabel}
              </p>
            )}
          </div>

          <div className="flex flex-col items-end gap-1 text-right">
            {dueLabel && (
              <span
                className={cn(
                  "text-sm",
                  isOverdue ? "font-medium text-destructive" : "text-muted-foreground"
                )}
              >
                {dueLabel}
              </span>
            )}
            {isCompleted && action.completedAt && (
              <span className="text-xs text-muted-foreground">
                Done {formatDueDate(action.completedAt)}
              </span>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

/** Format a date string into a human-readable label. */
function formatDueDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.floor(
      (date.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Tomorrow";
    if (diffDays === -1) return "Yesterday";
    if (diffDays < -1) return `${Math.abs(diffDays)} days ago`;
    if (diffDays <= 7) return `In ${diffDays} days`;

    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}
