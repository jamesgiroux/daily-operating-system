/**
 * IntelligenceCorrection — binary yes/no validation for AI-authored claims.
 *
 * The prompt stays neutral: "Is this accurate?". The actions are explicit:
 *   - Yes → record confirmation / reward the source
 *   - No  → record dismissal / hide the claim from the current UI
 *
 * `onConfirmed` / `onDismissed` run after the backend correction succeeds, so
 * parents can optimistically hide cards, clear JSON fields, or archive AI
 * suggestions without duplicating the feedback plumbing.
 */
import { useCallback } from "react";
import { useIntelligenceCorrection } from "@/hooks/useIntelligenceCorrection";
import { AccuracyPrompt, type AccuracyPromptOutcome } from "./AccuracyPrompt";

export interface IntelligenceCorrectionProps {
  /** Entity the AI assessment belongs to. */
  entityId: string;
  /** Always "account" today — the component is account-only per DOS-41. */
  entityType: "account";
  /** Field key the correction targets (e.g. "state_of_play", "health"). */
  field: string;
  /** Stable claim key for suppression tombstones on dismiss. */
  itemKey?: string | null;
  /** Extra side effects after a successful confirmation. */
  onConfirmed?: () => void | Promise<void>;
  /** Extra side effects after a successful dismissal. */
  onDismissed?: () => void | Promise<void>;
  /** Optional label override (default: "Is this accurate?"). */
  prompt?: string;
  /** Override for the post-submit confirmation copy. */
  doneLabel?: string;
}

/**
 * Inline correction prompt rendered after an AI assessment.
 *
 * State machine:
 *   idle      → "Is this accurate?" + Yes / No buttons
 *   Yes       → submits `confirmed` → optional parent callback → done
 *   No        → submits `dismissed` → optional parent callback → done
 *   done      → small "Recorded" affordance, click Undo to reset
 */
export function IntelligenceCorrection({
  entityId,
  entityType,
  field,
  itemKey,
  onConfirmed,
  onDismissed,
  prompt = "Is this accurate?",
  doneLabel = "Recorded.",
}: IntelligenceCorrectionProps) {
  const { submit, submitting, reset } = useIntelligenceCorrection();

  const onYes = useCallback(async () => {
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "confirmed",
    });
    if (ok) {
      await onConfirmed?.();
    }
    return ok;
  }, [entityId, entityType, field, onConfirmed, submit]);

  const onNo = useCallback(async () => {
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "dismissed",
      itemKey,
    });
    if (ok) {
      await onDismissed?.();
    }
    return ok;
  }, [entityId, entityType, field, itemKey, onDismissed, submit]);

  return (
    <AccuracyPrompt
      prompt={prompt}
      doneLabel={doneLabel}
      submitting={submitting}
      onUndo={reset}
      onYes={async (): Promise<AccuracyPromptOutcome> => {
        return (await onYes()) ? "done" : "stay";
      }}
      onNo={async (): Promise<AccuracyPromptOutcome> => {
        return (await onNo()) ? "done" : "stay";
      }}
    />
  );
}
