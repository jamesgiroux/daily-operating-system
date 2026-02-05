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
            : "Run the daily briefing workflow now"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Icon-only version for compact layouts
 */
export function RunNowIconButton({
  onClick,
  isRunning,
  disabled,
  className,
}: Omit<RunNowButtonProps, "variant" | "size">) {
  const isDisabled = isRunning || disabled;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClick}
            disabled={isDisabled}
            className={cn("h-8 w-8", className)}
          >
            {isRunning ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Play className="h-4 w-4" />
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {isRunning
            ? "Workflow is currently running"
            : "Run the daily briefing workflow now"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
