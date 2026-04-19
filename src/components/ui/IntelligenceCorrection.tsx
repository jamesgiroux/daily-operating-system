/**
 * IntelligenceCorrection — DOS-41 consolidated correction UX.
 *
 * Replaces the legacy thumbs up/down `IntelligenceFeedback` for AI-generated
 * summaries on **account pages only**. The interaction model collapses three
 * previously fragmented mechanisms (thumbs vote, inline edit, conflict
 * accept/dismiss) into a single "Is this accurate?" prompt with three
 * branches:
 *
 *   - Yes        → `confirmed`  (rewards source — Bayesian alpha++)
 *   - Partially  → `annotated`  (user note threaded into next intel prompt)
 *   - No         → `corrected`  (replaces value, penalizes source, recomputes
 *                                health when the field is health-affecting —
 *                                see DOS-227 in services/feedback.rs)
 *
 * All three actions flow through `submit_intelligence_correction` via the
 * `useIntelligenceCorrection` hook, which is the stable backend surface.
 *
 * Gradual rollout (per DOS-41 spec): this component is account-page only.
 * `IntelligenceFeedback` remains on project + person surfaces.
 */
import { useCallback, useState } from "react";
import { useIntelligenceCorrection } from "@/hooks/useIntelligenceCorrection";
import styles from "./IntelligenceCorrection.module.css";

export interface IntelligenceCorrectionProps {
  /** Entity the AI assessment belongs to. */
  entityId: string;
  /** Always "account" today — the component is account-only per DOS-41. */
  entityType: "account";
  /** Field key the correction targets (e.g. "state_of_play", "health"). */
  field: string;
  /**
   * Current value rendered to the user. Used as the seed for the inline
   * editor when the user clicks "No" so they can edit-in-place rather
   * than retype from scratch. Optional — `corrected` still works when
   * the surface doesn't have a stable scalar (e.g. multi-paragraph blob).
   */
  currentValue?: string | null;
  /**
   * Optional: called immediately after a successful `corrected` submission
   * with the new value. Lets the parent surface update its rendered state
   * without waiting for the next intelligence refresh — DOS-41 acceptance
   * criterion 7 ("Corrections visible to the user — field updates
   * immediately, not on next enrichment").
   */
  onCorrected?: (correctedValue: string) => void;
  /** Optional label override (default: "Is this accurate?"). */
  prompt?: string;
}

type Mode = "idle" | "annotating" | "correcting" | "done";

/**
 * Inline correction prompt rendered after an AI assessment.
 *
 * State machine:
 *   idle      → "Is this accurate?" + Yes / Partially / No buttons
 *   Yes       → submits `confirmed` → done
 *   Partially → annotating (textarea) → submits `annotated` → done
 *   No        → correcting (textarea seeded with currentValue) → submits
 *               `corrected` → done (and notifies parent via onCorrected)
 *   done      → small "Recorded" affordance, click Undo to reset
 */
export function IntelligenceCorrection({
  entityId,
  entityType,
  field,
  currentValue,
  onCorrected,
  prompt = "Is this accurate?",
}: IntelligenceCorrectionProps) {
  const { submit, submitting, reset } = useIntelligenceCorrection();
  const [mode, setMode] = useState<Mode>("idle");
  const [draft, setDraft] = useState<string>("");

  const onYes = useCallback(async () => {
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "confirmed",
    });
    if (ok) setMode("done");
  }, [entityId, entityType, field, submit]);

  const onPartially = useCallback(() => {
    setDraft("");
    setMode("annotating");
  }, []);

  const onNo = useCallback(() => {
    setDraft(currentValue ?? "");
    setMode("correcting");
  }, [currentValue]);

  const submitAnnotation = useCallback(async () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "annotated",
      annotation: trimmed,
    });
    if (ok) setMode("done");
  }, [draft, entityId, entityType, field, submit]);

  const submitCorrection = useCallback(async () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "corrected",
      correctedValue: trimmed,
    });
    if (ok) {
      setMode("done");
      onCorrected?.(trimmed);
    }
  }, [draft, entityId, entityType, field, submit, onCorrected]);

  const cancel = useCallback(() => {
    setDraft("");
    setMode("idle");
  }, []);

  const startOver = useCallback(() => {
    reset();
    setDraft("");
    setMode("idle");
  }, [reset]);

  if (mode === "done") {
    return (
      <span className={styles.wrapper}>
        <span className={styles.done}>
          Recorded.{" "}
          <button
            type="button"
            className={styles.linkButton}
            onClick={startOver}
          >
            Undo
          </button>
        </span>
      </span>
    );
  }

  if (mode === "annotating" || mode === "correcting") {
    const isCorrection = mode === "correcting";
    return (
      <span className={styles.wrapper}>
        <span className={styles.editor}>
          <span className={styles.editorLabel}>
            {isCorrection ? "What should it say?" : "What's missing or off?"}
          </span>
          <textarea
            className={styles.textarea}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            // eslint-disable-next-line jsx-a11y/no-autofocus
            autoFocus
            rows={isCorrection ? 3 : 2}
            placeholder={
              isCorrection
                ? "Replacement value"
                : "Add the nuance the model missed"
            }
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                cancel();
              } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                if (isCorrection) {
                  void submitCorrection();
                } else {
                  void submitAnnotation();
                }
              }
            }}
          />
          <span className={styles.editorActions}>
            <button
              type="button"
              className={styles.primaryButton}
              disabled={submitting || !draft.trim()}
              onClick={isCorrection ? submitCorrection : submitAnnotation}
            >
              {submitting ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              className={styles.linkButton}
              disabled={submitting}
              onClick={cancel}
            >
              Cancel
            </button>
          </span>
        </span>
      </span>
    );
  }

  return (
    <span className={styles.wrapper}>
      <span className={styles.prompt}>{prompt}</span>
      <span className={styles.choices}>
        <button
          type="button"
          className={styles.choice}
          onClick={onYes}
          disabled={submitting}
          aria-label="Mark this assessment as accurate"
        >
          Yes
        </button>
        <button
          type="button"
          className={styles.choice}
          onClick={onPartially}
          disabled={submitting}
          aria-label="Add a note about what the model missed"
        >
          Partially
        </button>
        <button
          type="button"
          className={styles.choice}
          onClick={onNo}
          disabled={submitting}
          aria-label="Open inline editor to replace this value"
        >
          No
        </button>
      </span>
    </span>
  );
}
