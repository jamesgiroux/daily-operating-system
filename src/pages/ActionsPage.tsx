import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { Action, Priority } from "@/types";
import { cn } from "@/lib/utils";
import { AlertCircle, Clock, CheckCircle2, AlertTriangle } from "lucide-react";

type FilterTab = "all" | "overdue" | "today" | "week" | "waiting";

interface ActionsResult {
  status: "success" | "empty" | "error";
  data?: Action[];
  message?: string;
}

const priorityStyles: Record<Priority, string> = {
  P1: "bg-destructive/15 text-destructive border-destructive/30",
  P2: "bg-primary/15 text-primary border-primary/30",
  P3: "bg-muted text-muted-foreground border-muted-foreground/30",
};

const priorityLabels: Record<Priority, string> = {
  P1: "Critical",
  P2: "High",
  P3: "Normal",
};

export default function ActionsPage() {
  const [actions, setActions] = useState<Action[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<FilterTab>("all");

  useEffect(() => {
    async function loadActions() {
      try {
        const result = await invoke<ActionsResult>("get_all_actions");
        if (result.status === "success" && result.data) {
          setActions(result.data);
        } else if (result.status === "empty") {
          setActions([]);
        } else if (result.status === "error") {
          setError(result.message || "Failed to load actions");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadActions();
  }, []);

  const filteredActions = actions.filter((action) => {
    switch (activeTab) {
      case "overdue":
        return action.isOverdue;
      case "today":
        return action.dueDate === "Today";
      case "week":
        return !action.isOverdue && action.status === "pending";
      case "waiting":
        // For now, no waiting filter in the data model
        return false;
      default:
        return true;
    }
  });

  const counts = {
    all: actions.length,
    overdue: actions.filter((a) => a.isOverdue).length,
    today: actions.filter((a) => a.dueDate === "Today").length,
    week: actions.filter((a) => !a.isOverdue && a.status === "pending").length,
    waiting: 0,
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
      <main className="flex-1 overflow-hidden p-6">
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error}</p>
            </div>
          </CardContent>
        </Card>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">Actions</h1>
            <p className="text-sm text-muted-foreground">
              All action items with context and source
            </p>
          </div>

          {/* Filter tabs */}
          <div className="mb-6 flex gap-2">
            {(["all", "overdue", "today", "week"] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={cn(
                  "inline-flex items-center gap-2 rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                  activeTab === tab
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted hover:bg-muted/80"
                )}
              >
                {tab === "overdue" && <AlertTriangle className="size-3.5" />}
                {tab === "today" && <Clock className="size-3.5" />}
                {tab.charAt(0).toUpperCase() + tab.slice(1)}
                {counts[tab] > 0 && (
                  <span
                    className={cn(
                      "rounded-full px-1.5 py-0.5 text-xs",
                      activeTab === tab
                        ? "bg-primary-foreground/20"
                        : "bg-background"
                    )}
                  >
                    {counts[tab]}
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* Actions list */}
          <div className="space-y-3">
            {filteredActions.length === 0 ? (
              <Card>
                <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                  <CheckCircle2 className="mb-4 size-12 text-success" />
                  <p className="text-lg font-medium">No actions to show</p>
                  <p className="text-sm text-muted-foreground">
                    {activeTab === "overdue"
                      ? "Great job! No overdue items."
                      : activeTab === "today"
                        ? "Nothing due today."
                        : "You're all caught up!"}
                  </p>
                </CardContent>
              </Card>
            ) : (
              filteredActions.map((action) => (
                <ActionCard key={action.id} action={action} />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function ActionCard({ action }: { action: Action }) {
  return (
    <Card
      className={cn(
        "transition-all hover:-translate-y-0.5 hover:shadow-md",
        action.isOverdue && "border-l-4 border-l-destructive"
      )}
    >
      <CardContent className="p-5">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-2">
              <h3 className="font-medium">{action.title}</h3>
              <Badge
                variant="outline"
                className={cn("text-xs", priorityStyles[action.priority])}
              >
                {priorityLabels[action.priority]}
              </Badge>
            </div>

            {action.account && (
              <p className="text-sm text-primary">{action.account}</p>
            )}

            {action.context && (
              <p className="text-sm text-muted-foreground">{action.context}</p>
            )}

            {action.source && (
              <p className="text-xs text-muted-foreground/70">
                Source: {action.source}
              </p>
            )}
          </div>

          <div className="flex flex-col items-end gap-1 text-right">
            {action.dueDate && (
              <span
                className={cn(
                  "text-sm",
                  action.isOverdue ? "text-destructive font-medium" : "text-muted-foreground"
                )}
              >
                {action.dueDate}
              </span>
            )}
            {action.daysOverdue && action.daysOverdue > 0 && (
              <span className="text-xs text-destructive">
                {action.daysOverdue} days overdue
              </span>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
