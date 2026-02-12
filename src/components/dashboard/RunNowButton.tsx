import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Play, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface RunNowButtonProps {
  onClick: () => void;
  isRunning: boolean;
  disabled?: boolean;
  aiUnavailable?: boolean;
  className?: string;
  variant?: "default" | "ghost" | "outline";
  size?: "default" | "sm" | "lg" | "icon";
}

/**
 * Button to manually trigger workflow execution
 *
 * Disables during execution and shows loading spinner.
 * Provides tooltip explaining the action.
 */
export function RunNowButton({
  onClick,
  isRunning,
  disabled,
  aiUnavailable,
  className,
  variant = "outline",
  size = "sm",
}: RunNowButtonProps) {
  const isDisabled = isRunning || disabled;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant={variant}
            size={size}
            onClick={onClick}
            disabled={isDisabled}
            className={cn("gap-2", className)}
          >
            {isRunning ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Running...
              </>
            ) : (
              <>
                <Play className="h-4 w-4" />
                Run Now
              </>
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {isRunning
            ? "Workflow is currently running"
            : aiUnavailable
            ? "Run the daily briefing in base prep mode"
            : "Run the daily briefing workflow now"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Compact version with icon + label for the header
 */
export function RunNowIconButton({
  onClick,
  isRunning,
  disabled,
  aiUnavailable,
  className,
}: Omit<RunNowButtonProps, "variant" | "size">) {
  const isDisabled = isRunning || disabled;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClick}
            disabled={isDisabled}
            className={cn("h-8 items-center gap-1.5 px-2.5 text-muted-foreground", className)}
          >
            {isRunning ? (
              <>
                <Loader2 className="size-3.5 shrink-0 animate-spin" />
                <span className="text-xs leading-none">Running...</span>
              </>
            ) : (
              <>
                <Play className="size-3.5 shrink-0" />
                <span className="text-xs leading-none">Run Briefing</span>
              </>
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {isRunning
            ? "Workflow is currently running"
            : aiUnavailable
            ? "Run the daily briefing in base prep mode"
            : "Run the daily briefing workflow now"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
