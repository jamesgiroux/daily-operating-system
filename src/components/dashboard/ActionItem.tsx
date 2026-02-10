import { Link } from "@tanstack/react-router";
import { Circle, CheckCircle2, AlertCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { Action, Priority } from "@/types";
import { cn, stripMarkdown } from "@/lib/utils";

interface ActionItemProps {
  action: Action;
  isLocallyCompleted?: boolean;
  onComplete?: (id: string) => void;
}

const priorityStyles: Record<Priority, string> = {
  P1: "bg-destructive/15 text-destructive",
  P2: "bg-primary/15 text-primary",
  P3: "bg-muted text-muted-foreground",
};

export function ActionItem({ action, isLocallyCompleted, onComplete }: ActionItemProps) {
  const isCompleted = action.status === "completed" || isLocallyCompleted;

  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-md p-3 transition-all duration-150",
        "hover:bg-muted/50",
        isCompleted && "bg-muted/20 opacity-60"
      )}
    >
      <button
        onClick={() => {
          if (!isCompleted && onComplete) {
            onComplete(action.id);
          }
        }}
        disabled={isCompleted}
        className={cn(
          "mt-0.5 shrink-0 transition-colors",
          isCompleted
            ? "text-success"
            : "text-muted-foreground hover:text-foreground cursor-pointer"
        )}
        aria-label={isCompleted ? "Completed" : "Mark as complete"}
      >
        {isCompleted ? (
          <CheckCircle2 className="size-5" />
        ) : (
          <Circle className="size-5" />
        )}
      </button>

      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-start justify-between gap-2">
          <Link
            to="/actions/$actionId"
            params={{ actionId: action.id }}
            className={cn(
              "font-medium transition-colors hover:text-primary",
              isCompleted && "line-through text-muted-foreground"
            )}
          >
            {stripMarkdown(action.title)}
          </Link>
          <Badge className={cn("shrink-0", priorityStyles[action.priority])} variant="secondary">
            {action.priority}
          </Badge>
        </div>

        {action.context && (
          <p className="text-sm text-muted-foreground line-clamp-1">{action.context}</p>
        )}

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
