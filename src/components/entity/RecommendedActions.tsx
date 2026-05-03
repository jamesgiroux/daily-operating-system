/**
 * RecommendedActions — AI-recommended actions from intelligence enrichment.
 *
 * Renders recommended actions with Track (accept) and Dismiss buttons.
 * Each card shows title, rationale, priority label, and optional suggested due date.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { RecommendedAction } from "@/types";
import s from "./RecommendedActions.module.css";

interface RecommendedActionsProps {
  entityId: string;
  entityType: string;
  actions: RecommendedAction[];
  onRefresh?: () => Promise<void> | void;
}

function priorityLabel(priority: number): string {
  switch (priority) {
    case 1:
      return "Urgent";
    case 2:
      return "High";
    case 4:
      return "Low";
    default:
      return "Medium";
  }
}

function priorityColor(priority: number): string {
  switch (priority) {
    case 1:
      return "var(--color-spice-terracotta)";
    case 2:
      return "var(--color-spice-turmeric)";
    default:
      return "var(--color-text-tertiary)";
  }
}

export function RecommendedActions({
  entityId,
  entityType,
  actions,
  onRefresh,
}: RecommendedActionsProps) {
  const [dismissed, setDismissed] = useState<Set<number>>(new Set());
  const [tracked, setTracked] = useState<Set<number>>(new Set());

  const handleTrack = useCallback(
    async (index: number) => {
      try {
        await invoke("track_recommendation", {
          entityId,
          entityType,
          index,
        });
        setTracked((prev) => new Set(prev).add(index));
        toast.success("Action tracked");
        onRefresh?.();
      } catch (err) {
        console.error("Failed to track recommendation:", err);
        toast.error("Failed to track recommendation");
      }
    },
    [entityId, entityType, onRefresh],
  );

  const handleDismiss = useCallback(
    async (index: number) => {
      try {
        // Compute adjusted index: dismissed items shift indices down
        const adjustedIndex = computeAdjustedIndex(index, dismissed);
        await invoke("dismiss_recommendation", {
          entityId,
          entityType,
          index: adjustedIndex,
        });
        setDismissed((prev) => new Set(prev).add(index));
        onRefresh?.();
      } catch (err) {
        console.error("Failed to dismiss recommendation:", err);
        toast.error("Failed to dismiss recommendation");
      }
    },
    [entityId, entityType, dismissed, onRefresh],
  );

  const visibleActions = actions
    .map((action, i) => ({ action, originalIndex: i }))
    .filter(({ originalIndex }) => !dismissed.has(originalIndex) && !tracked.has(originalIndex));

  if (visibleActions.length === 0) return null;

  return (
    <div className={s.root}>
      <div className={s.label}>
        Recommended
      </div>
      {visibleActions.map(({ action, originalIndex }) => (
        <div key={originalIndex} className={s.action}>
          <div className={s.content}>
            <div className={s.metaRow}>
              <span
                className={s.priority}
                // Runtime priority determines the status color.
                style={{
                  color: priorityColor(action.priority),
                }}
              >
                {priorityLabel(action.priority)}
              </span>
              {action.suggestedDue && (
                <span className={s.dueDate}>
                  Due {action.suggestedDue}
                </span>
              )}
            </div>
            <div className={s.title}>
              {action.title}
            </div>
            <div className={s.rationale}>
              {action.rationale}
            </div>
            <div className={s.source}>
              Based on account intelligence
            </div>
          </div>

          <div className={s.actions}>
            <button
              onClick={() => handleTrack(originalIndex)}
              title="Track"
              className={`${s.iconButton} ${s.trackButton}`}
            >
              <svg width={14} height={14} viewBox="0 0 14 14" fill="none">
                <path
                  d="M3 7L6 10L11 4"
                  stroke="var(--color-garden-sage)"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </button>
            <button
              onClick={() => handleDismiss(originalIndex)}
              title="Dismiss"
              className={`${s.iconButton} ${s.dismissButton}`}
            >
              <svg width={14} height={14} viewBox="0 0 14 14" fill="none">
                <path
                  d="M4 4L10 10M10 4L4 10"
                  stroke="var(--color-spice-terracotta)"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </svg>
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * Compute the adjusted index for dismiss operations.
 * Since each dismiss removes an item from the backend array,
 * we need to account for already-dismissed items with lower indices.
 */
function computeAdjustedIndex(originalIndex: number, dismissed: Set<number>): number {
  let offset = 0;
  for (const d of dismissed) {
    if (d < originalIndex) offset++;
  }
  return originalIndex - offset;
}
