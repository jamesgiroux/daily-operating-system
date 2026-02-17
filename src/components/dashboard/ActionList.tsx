import { useState, useCallback, useEffect } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { CheckSquare, ChevronRight, Sparkles } from "lucide-react";
import { ActionItem } from "./ActionItem";
import type { Action } from "@/types";

interface ActionListProps {
  actions: Action[];
  maxVisible?: number;
}

export function ActionList({ actions, maxVisible = 5 }: ActionListProps) {
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());
  const [proposedActions, setProposedActions] = useState<Action[]>([]);
  const [dismissedIds, setDismissedIds] = useState<Set<string>>(new Set());

  // Load proposed actions from the backend
  useEffect(() => {
    invoke<Array<{
      id: string;
      title: string;
      priority: string;
      status: string;
      accountId?: string;
      dueDate?: string;
      context?: string;
      sourceLabel?: string;
    }>>("get_proposed_actions")
      .then((result) => {
        const mapped: Action[] = result.map((a) => ({
          id: a.id,
          title: a.title,
          priority: (a.priority || "P2") as Action["priority"],
          status: "proposed" as const,
          account: a.accountId,
          dueDate: a.dueDate,
          context: a.context,
          source: a.sourceLabel,
        }));
        setProposedActions(mapped);
      })
      .catch(() => {
        // Silent — proposed actions are optional
      });
  }, []);

  const handleComplete = useCallback((id: string) => {
    setCompletedIds((prev) => new Set(prev).add(id));
    invoke("complete_action", { id }).catch(() => {});
  }, []);

  const handleAccept = useCallback((id: string) => {
    setDismissedIds((prev) => new Set(prev).add(id));
    invoke("accept_proposed_action", { id }).catch(() => {});
  }, []);

  const handleReject = useCallback((id: string) => {
    setDismissedIds((prev) => new Set(prev).add(id));
    invoke("reject_proposed_action", { id }).catch(() => {});
  }, []);

  // Get pending actions sorted by priority (P1 first, then P2, then P3)
  const pendingActions = actions
    .filter((a) => a.status !== "completed" && a.status !== "proposed" && !completedIds.has(a.id))
    .sort((a, b) => {
      const priorityOrder = { P1: 0, P2: 1, P3: 2 };
      return priorityOrder[a.priority] - priorityOrder[b.priority];
    });

  const visibleProposed = proposedActions.filter((a) => !dismissedIds.has(a.id));

  const pendingCount = pendingActions.length;
  const visibleActions = pendingActions.slice(0, maxVisible);
  const hasMore = pendingCount > maxVisible;

  return (
    <section>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Actions Due
        </h3>
        <span className="font-mono text-xs font-light text-muted-foreground">
          {pendingCount} {pendingCount === 1 ? "item" : "items"}
        </span>
      </div>

      {/* AI Suggested actions */}
      {visibleProposed.length > 0 && (
        <div className="mb-3">
          <div className="flex items-center gap-1.5 mb-2">
            <Sparkles className="size-3.5 text-primary/60" />
            <span className="text-xs font-medium text-primary/60 uppercase tracking-wider">
              Suggested
            </span>
          </div>
          <div className="space-y-2">
            {visibleProposed.slice(0, 3).map((action) => (
              <ActionItem
                key={action.id}
                action={action}
                onAccept={handleAccept}
                onReject={handleReject}
              />
            ))}
          </div>
        </div>
      )}

      {pendingCount === 0 && visibleProposed.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-6 text-center">
          <CheckSquare className="mb-2 size-8 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">No actions due</p>
        </div>
      ) : pendingCount === 0 ? null : (
        <div className="space-y-2">
          {visibleActions.map((action) => (
            <ActionItem
              key={action.id}
              action={action}
              isLocallyCompleted={completedIds.has(action.id)}
              onComplete={handleComplete}
            />
          ))}

          {hasMore && (
            <Link
              to="/actions"
              search={{ search: undefined }}
              className="flex w-full items-center justify-center gap-1 py-2 text-sm text-primary hover:text-primary/80 transition-colors"
            >
              View all {pendingCount} actions
              <ChevronRight className="size-4" />
            </Link>
          )}
        </div>
      )}
    </section>
  );
}
