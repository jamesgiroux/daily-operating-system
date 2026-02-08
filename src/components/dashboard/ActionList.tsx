import { useState, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { CheckSquare, ChevronRight } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ActionItem } from "./ActionItem";
import type { Action } from "@/types";

interface ActionListProps {
  actions: Action[];
  maxVisible?: number;
}

export function ActionList({ actions, maxVisible = 5 }: ActionListProps) {
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());

  const handleComplete = useCallback((id: string) => {
    // Optimistic UI — mark complete immediately
    setCompletedIds((prev) => new Set(prev).add(id));

    // Try to persist to SQLite (best-effort — action may not be synced yet)
    invoke("complete_action", { id }).catch(() => {
      // Silent — the visual completion still holds for this session
    });
  }, []);

  // Get pending actions sorted by priority (P1 first, then P2, then P3)
  const pendingActions = actions
    .filter((a) => a.status !== "completed" && !completedIds.has(a.id))
    .sort((a, b) => {
      const priorityOrder = { P1: 0, P2: 1, P3: 2 };
      return priorityOrder[a.priority] - priorityOrder[b.priority];
    });

  const pendingCount = pendingActions.length;
  const visibleActions = pendingActions.slice(0, maxVisible);
  const hasMore = pendingCount > maxVisible;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-medium">
            Actions Due
          </CardTitle>
          <span className="font-mono text-sm font-light text-muted-foreground">
            {pendingCount} {pendingCount === 1 ? "item" : "items"}
          </span>
        </div>
      </CardHeader>
      <CardContent>
        {pendingCount === 0 ? (
          <div className="flex flex-col items-center justify-center py-6 text-center">
            <CheckSquare className="mb-2 size-8 text-muted-foreground/50" />
            <p className="text-sm text-muted-foreground">No actions due</p>
          </div>
        ) : (
          <div className="space-y-3">
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
      </CardContent>
    </Card>
  );
}
