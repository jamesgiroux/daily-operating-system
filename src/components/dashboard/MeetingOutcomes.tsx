import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Trophy,
  AlertTriangle,
  CircleDot,
  Check,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { MeetingOutcomeData, DbAction } from "@/types";
import { cn } from "@/lib/utils";

interface MeetingOutcomesProps {
  outcomes: MeetingOutcomeData;
  onRefresh: () => void;
}

export function MeetingOutcomes({ outcomes, onRefresh }: MeetingOutcomesProps) {
  return (
    <div className="space-y-4 text-sm">
      {/* Summary */}
      {outcomes.summary && (
        <p className="text-muted-foreground">{outcomes.summary}</p>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {/* Wins */}
        {outcomes.wins.length > 0 && (
          <OutcomeSection
            title="Wins"
            icon={<Trophy className="size-3 text-success" />}
            items={outcomes.wins}
            type="win"
            onRefresh={onRefresh}
          />
        )}

        {/* Risks */}
        {outcomes.risks.length > 0 && (
          <OutcomeSection
            title="Risks"
            icon={<AlertTriangle className="size-3 text-peach" />}
            items={outcomes.risks}
            type="risk"
            onRefresh={onRefresh}
          />
        )}

        {/* Decisions */}
        {outcomes.decisions.length > 0 && (
          <OutcomeSection
            title="Decisions"
            icon={<CircleDot className="size-3 text-primary" />}
            items={outcomes.decisions}
            type="decision"
            onRefresh={onRefresh}
          />
        )}
      </div>

      {/* Actions */}
      {outcomes.actions.length > 0 && (
        <div className="space-y-1.5">
          <h4 className="font-medium">Actions</h4>
          <div className="space-y-1">
            {outcomes.actions.map((action) => (
              <ActionRow
                key={action.id}
                action={action}
                onRefresh={onRefresh}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function OutcomeSection({
  title,
  icon,
  items,
  type: _type,
  onRefresh: _onRefresh,
}: {
  title: string;
  icon: React.ReactNode;
  items: string[];
  type: string;
  onRefresh: () => void;
}) {
  return (
    <div className="space-y-1.5">
      <h4 className="flex items-center gap-1.5 font-medium">
        {icon}
        {title}
        <span className="text-xs font-normal text-muted-foreground">
          ({items.length})
        </span>
      </h4>
      <ul className="space-y-1">
        {items.map((item, i) => (
          <li
            key={i}
            className="text-muted-foreground"
          >
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}

function ActionRow({
  action,
  onRefresh,
}: {
  action: DbAction;
  onRefresh: () => void;
}) {
  const isCompleted = action.status === "completed";

  const handleComplete = useCallback(async () => {
    try {
      if (isCompleted) {
        await invoke("reopen_action", { id: action.id });
      } else {
        await invoke("complete_action", { id: action.id });
      }
      onRefresh();
    } catch (err) {
      console.error("Failed to toggle action:", err);
    }
  }, [action.id, isCompleted, onRefresh]);

  const handleCyclePriority = useCallback(async () => {
    const cycle: Record<string, string> = { P1: "P2", P2: "P3", P3: "P1" };
    const next = cycle[action.priority] || "P2";
    try {
      await invoke("update_action_priority", {
        id: action.id,
        priority: next,
      });
      onRefresh();
    } catch (err) {
      console.error("Failed to update priority:", err);
    }
  }, [action.id, action.priority, onRefresh]);

  return (
    <div className="flex items-center gap-2 rounded px-1 py-0.5">
      <button
        onClick={handleComplete}
        className={cn(
          "flex size-4 shrink-0 items-center justify-center rounded border",
          isCompleted
            ? "border-success bg-success/20 text-success"
            : "border-muted-foreground/40"
        )}
      >
        {isCompleted && <Check className="size-3" />}
      </button>

      <Badge
        variant="outline"
        className={cn(
          "cursor-pointer px-1 py-0 text-[10px]",
          action.priority === "P1" && "border-peach/50 text-peach",
          action.priority === "P3" && "border-muted-foreground/30 text-muted-foreground"
        )}
        onClick={handleCyclePriority}
      >
        {action.priority}
      </Badge>

      <span
        className={cn(
          "flex-1 text-xs",
          isCompleted && "text-muted-foreground line-through"
        )}
      >
        {action.title}
      </span>

      {action.dueDate && (
        <span className="text-[10px] text-muted-foreground">
          {action.dueDate}
        </span>
      )}
    </div>
  );
}
