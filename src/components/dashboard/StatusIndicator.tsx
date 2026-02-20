import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  type WorkflowStatus,
  getPhaseDescription,
  formatDuration,
} from "@/hooks/useWorkflow";
import { Loader2, CheckCircle, XCircle, Clock } from "lucide-react";

interface StatusIndicatorProps {
  status: WorkflowStatus;
  nextRunTime?: string | null;
  aiUnavailable?: boolean;
  className?: string;
}

/**
 * Visual indicator for workflow status
 *
 * Shows:
 * - Idle: Gray badge, next run time in tooltip
 * - Running: Animated gold badge with phase
 * - Completed: Green badge with duration
 * - Failed: Red badge with error hint
 */
export function StatusIndicator({
  status,
  nextRunTime,
  aiUnavailable,
  className,
}: StatusIndicatorProps) {
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={cn("inline-flex items-center", className)}>
            <StatusBadge status={status} />
          </div>
        </TooltipTrigger>
        <TooltipContent side="bottom" className="max-w-xs">
          <StatusTooltip
            status={status}
            nextRunTime={nextRunTime}
            aiUnavailable={aiUnavailable}
          />
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

function StatusBadge({ status }: { status: WorkflowStatus }) {
  switch (status.status) {
    case "idle":
      return (
        <Badge variant="secondary" className="gap-1.5">
          <Clock className="h-3 w-3" />
          Idle
        </Badge>
      );

    case "running":
      return (
        <Badge className="gap-1.5 bg-gold text-charcoal hover:bg-gold/90">
          <Loader2 className="h-3 w-3 animate-spin" />
          {getPhaseLabel(status.phase)}
        </Badge>
      );

    case "completed":
      return (
        <Badge className="gap-1.5 bg-sage/15 text-sage hover:bg-sage/20">
          <CheckCircle className="h-3 w-3" />
          Ready
        </Badge>
      );

    case "failed":
      return (
        <Badge variant="destructive" className="gap-1.5">
          <XCircle className="h-3 w-3" />
          Error
        </Badge>
      );
  }
}

function StatusTooltip({
  status,
  nextRunTime,
  aiUnavailable,
}: {
  status: WorkflowStatus;
  nextRunTime?: string | null;
  aiUnavailable?: boolean;
}) {
  switch (status.status) {
    case "idle":
      return (
        <div className="space-y-1">
          <p className="font-medium">Waiting for next run</p>
          {nextRunTime && (
            <p className="text-muted-foreground text-sm">
              Next: {formatNextRunTime(nextRunTime)}
            </p>
          )}
          {aiUnavailable && (
            <p className="text-muted-foreground text-xs">
              AI unavailable. Briefing will run in base mode.
            </p>
          )}
        </div>
      );

    case "running":
      return (
        <div className="space-y-1">
          <p className="font-medium">{getPhaseDescription(status.phase)}</p>
          <p className="text-muted-foreground text-sm">
            Started {formatStartTime(status.startedAt)}
          </p>
          {aiUnavailable && status.phase === "enriching" && (
            <p className="text-muted-foreground text-xs">
              AI may be limited. Core schedule, actions, and prep still deliver.
            </p>
          )}
        </div>
      );

    case "completed":
      return (
        <div className="space-y-1">
          <p className="font-medium">Briefing ready</p>
          <p className="text-muted-foreground text-sm">
            Completed in {formatDuration(status.durationSecs)}
          </p>
          {aiUnavailable && (
            <p className="text-muted-foreground text-xs">
              Delivered in base prep mode.
            </p>
          )}
        </div>
      );

    case "failed":
      return (
        <div className="space-y-1">
          <p className="font-medium">Workflow failed</p>
          <p className="text-destructive text-sm">{status.error.message}</p>
          {status.error.canRetry && (
            <p className="text-muted-foreground text-xs">
              Click "Run Now" to retry
            </p>
          )}
          {aiUnavailable && (
            <p className="text-muted-foreground text-xs">
              You can still run a core briefing without AI.
            </p>
          )}
        </div>
      );
  }
}

function getPhaseLabel(phase: string): string {
  switch (phase) {
    case "preparing":
      return "Preparing";
    case "enriching":
      return "AI Processing";
    case "delivering":
      return "Delivering";
    default:
      return "Running";
  }
}

function formatNextRunTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diff = date.getTime() - now.getTime();

  // If less than an hour, show relative time
  if (diff < 60 * 60 * 1000) {
    const minutes = Math.round(diff / (60 * 1000));
    return `in ${minutes} minutes`;
  }

  // If today, show time only
  if (date.toDateString() === now.toDateString()) {
    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }

  // Otherwise show date and time
  return date.toLocaleDateString([], {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
  });
}

function formatStartTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  // Show relative time
  const seconds = Math.round(diff / 1000);
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  const minutes = Math.round(seconds / 60);
  return `${minutes}m ago`;
}
