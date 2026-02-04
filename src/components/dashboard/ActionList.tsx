import { CheckSquare } from "lucide-react";
import { ActionItem } from "./ActionItem";
import type { Action } from "@/types";
import { cn } from "@/lib/utils";

interface ActionListProps {
  actions: Action[];
}

export function ActionList({ actions }: ActionListProps) {
  // Group actions by status and priority
  const overdueActions = actions.filter(
    (a) => a.isOverdue && a.status !== "completed"
  );
  const todayActions = actions.filter(
    (a) => !a.isOverdue && a.dueDate === "Today" && a.status !== "completed"
  );
  const thisWeekActions = actions.filter(
    (a) =>
      !a.isOverdue &&
      a.dueDate !== "Today" &&
      a.status !== "completed"
  );
  const completedActions = actions.filter((a) => a.status === "completed");

  const pendingCount = actions.filter((a) => a.status !== "completed").length;

  if (actions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center">
        <CheckSquare className="mb-2 size-8 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">No actions today</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-lg font-semibold">
          <CheckSquare className="size-5" />
          Actions
        </h2>
        <span className="text-sm text-muted-foreground">
          {pendingCount} pending
        </span>
      </div>

      <div className="space-y-4">
        {overdueActions.length > 0 && (
          <ActionGroup
            title="Overdue"
            actions={overdueActions}
            className="text-destructive"
          />
        )}

        {todayActions.length > 0 && (
          <ActionGroup title="Due Today" actions={todayActions} />
        )}

        {thisWeekActions.length > 0 && (
          <ActionGroup title="This Week" actions={thisWeekActions} />
        )}

        {completedActions.length > 0 && (
          <ActionGroup
            title="Completed"
            actions={completedActions}
            className="text-muted-foreground"
          />
        )}
      </div>
    </div>
  );
}

function ActionGroup({
  title,
  actions,
  className,
}: {
  title: string;
  actions: Action[];
  className?: string;
}) {
  return (
    <div className="space-y-2">
      <h3 className={cn("text-sm font-medium", className)}>{title}</h3>
      <div className="space-y-2">
        {actions.map((action) => (
          <ActionItem key={action.id} action={action} />
        ))}
      </div>
    </div>
  );
}
