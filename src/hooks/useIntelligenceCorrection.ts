import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

/**
 * DOS-41: Consolidated intelligence correction action.
 *
 * - `confirmed` — user agrees with the AI output (rewards the source)
 * - `rejected` — user disagrees but does not want the item hidden
 * - `corrected` — user replaces the AI output (penalizes source; triggers
 *   health recalc when the field is health-affecting on an account)
 * - `dismissed` — user marks the AI output wrong and wants it hidden
 */
export type CorrectionAction =
  | "confirmed"
  | "rejected"
  | "annotated"
  | "corrected"
  | "dismissed";

export interface SubmitCorrectionArgs {
  entityId: string;
  entityType: string;
  field: string;
  action: CorrectionAction;
  /** Stable claim key for suppression tombstones on dismiss. */
  itemKey?: string | null;
  /** Required for `corrected`; ignored for `confirmed` / `rejected` / `annotated`. */
  correctedValue?: string | null;
  /** User-authored note. Required for `annotated`; optional on others. */
  annotation?: string | null;
}

export interface UseIntelligenceCorrectionResult {
  /** True while a correction submission is in flight. */
  submitting: boolean;
  /** True once the most recent submission completed successfully. */
  success: boolean;
  /** Error message from the most recent submission, if any. */
  error: string | null;
  /**
   * Submit a correction. Resolves `true` on success, `false` on failure.
   * All supported action types go through `submit_intelligence_correction`.
   */
  submit: (args: SubmitCorrectionArgs) => Promise<boolean>;
  /** Reset `success` / `error` state (e.g. after dismissing a toast). */
  reset: () => void;
}

/**
 * DOS-41 hook — wraps the `submit_intelligence_correction` Tauri command.
 *
 * Component placement (`IntelligenceCorrection.tsx`) lands in Wave 1; this
 * hook is the stable backend surface that component will consume.
 *
 * Design notes:
 * - Validates action-specific required fields client-side so the Tauri
 *   round-trip isn't wasted on obvious mistakes.
 * - Surfaces toasts on failure (matches `useIntelligenceFeedback` UX).
 * - Exposes `loading` / `success` / `error` for optimistic UI affordances.
 */
export function useIntelligenceCorrection(): UseIntelligenceCorrectionResult {
  const [submitting, setSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = useCallback(
    async (args: SubmitCorrectionArgs): Promise<boolean> => {
      const {
        entityId,
        entityType,
        field,
        action,
        itemKey,
        correctedValue,
        annotation,
      } = args;

      // Client-side guards — keep parity with backend expectations.
      if (!entityId || !entityType || !field) {
        const msg = "Missing required correction target";
        setError(msg);
        setSuccess(false);
        toast.error(msg);
        return false;
      }
      if (action === "corrected" && !correctedValue) {
        const msg = "A corrected value is required";
        setError(msg);
        setSuccess(false);
        toast.error(msg);
        return false;
      }
      if (action === "annotated" && !annotation) {
        const msg = "A note is required to annotate";
        setError(msg);
        setSuccess(false);
        toast.error(msg);
        return false;
      }

      setSubmitting(true);
      setError(null);
      setSuccess(false);

      try {
        await invoke("submit_intelligence_correction", {
          request: {
            entityId,
            entityType,
            field,
            action,
            itemKey: itemKey ?? null,
            correctedValue: correctedValue ?? null,
            annotation: annotation ?? null,
          },
        });
        setSuccess(true);
        return true;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error("submit_intelligence_correction failed:", msg);
        setError(msg);
        toast.error("Could not save correction");
        return false;
      } finally {
        setSubmitting(false);
      }
    },
    [],
  );

  const reset = useCallback(() => {
    setSuccess(false);
    setError(null);
  }, []);

  return { submitting, success, error, submit, reset };
}
