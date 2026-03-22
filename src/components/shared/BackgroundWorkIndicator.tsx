/**
 * BackgroundWorkIndicator
 *
 * A subtle pulsing dot that signals background intelligence work is happening.
 * Visible only when active — invisible when idle.
 * Tooltip shows what's being updated.
 */

import type { BackgroundWorkState } from "@/hooks/useBackgroundStatus";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import styles from "./BackgroundWorkIndicator.module.css";

interface BackgroundWorkIndicatorProps {
  state: BackgroundWorkState;
}

export function BackgroundWorkIndicator({ state }: BackgroundWorkIndicatorProps) {
  if (state.phase === "idle") return null;

  const dotClass =
    state.phase === "started"
      ? styles.active
      : state.phase === "failed"
        ? styles.failed
        : styles.done;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={`${styles.dot} ${dotClass}`}
            aria-label={state.message || "Background work"}
          />
        </TooltipTrigger>
        <TooltipContent side="bottom" className="text-xs">
          {state.message || "Updating…"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
