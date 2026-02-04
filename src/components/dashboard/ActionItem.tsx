import { Circle, CheckCircle2, AlertCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { Action, Priority } from "@/types";
import { cn } from "@/lib/utils";

interface ActionItemProps {
  action: Action;
}

const priorityStyles: Record<Priority, string> = {
  P1: "bg-destructive/15 text-destructive",
  P2: "bg-primary/15 text-primary",
  P3: "bg-muted text-muted-foreground",
};

export function ActionItem({ action }: ActionItemProps) {
  const isCompleted = action.status === "completed";

  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border p-3 transition-all duration-150",
        "hover:bg-muted/30",
        isCompleted && "bg-muted/20 opacity-60"
      )}
    >
      <button
        className={cn(
          "mt-0.5 shrink-0 transition-colors",
          isCompleted ? "text-success" : "text-muted-foreground hover:text-foreground"
        )}
        aria-label={isCompleted ? "Mark as incomplete" : "Mark as complete"}
      >
        {isCompleted ? (
          <CheckCircle2 className="size-5" />
        ) : (
          <Circle className="size-5" />
        )}
      </button>

      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-start justify-between gap-2">
          <span
            className={cn(
              "font-medium",
              isCompleted && "line-through text-muted-foreground"
            )}
          >
            {action.title}
          </span>
          <Badge className={cn("shrink-0", priorityStyles[action.priority])} variant="secondary">
            {action.priority}
          </Badge>
        </div>

        <div className="flex flex-wrap items-center gap-2 text-sm">
          {action.account && (
            <span className="text-primary">{action.account}</span>
          )}
          {action.dueDate && (
            <span
              className={cn(
                "flex items-center gap-1",
                action.isOverdue
                  ? "text-destructive"
                  : "text-muted-foreground"
              )}
            >
              {action.isOverdue && <AlertCircle className="size-3" />}
              {action.dueDate}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
