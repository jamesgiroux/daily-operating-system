/**
 * IntelligenceCorrection — validation and correction for AI-authored claims.
 *
 * DOS-41: Replaces the three prior correction mechanisms (thumbs up/down,
 * inline edit, field-conflict accept/dismiss) with ONE unified "Is this
 * accurate?" pattern on account pages.
 *
 * Two variants:
 *
 *   variant="dismiss" (default) — binary Yes / No for claim cards:
 *     - Yes → `confirmed` (rewards source)
 *     - No  → `dismissed` (creates suppression tombstone; hides the card)
 *
 *   variant="correct" — three-state for AI summary/assessment fields:
 *     - Yes       → `confirmed` (rewards source)
 *     - Partially → annotation prompt → `annotated` (user context for next
 *                   intel pass)
 *     - No        → inline editor with previous value → `corrected`
 *                   (penalizes source, triggers health recalc on
 *                   health-affecting fields)
 *
 * `onConfirmed` / `onDismissed` / `onCorrected` fire after the backend
 * round-trip succeeds so parents can optimistically update UI.
 *
 * Corrections and annotations feed through the Intelligence Loop:
 *   1. Signals emitted (`intelligence_confirmed` / `intelligence_annotated` /
 *      `intelligence_corrected` / `intelligence_dismissed`)
 *   2. Bayesian source weights adjusted
 *   3. Health recalculated immediately on health-affecting fields
 *   4. Annotations threaded into next intel prompt as user context
 */
import { useCallback, useRef, useState } from "react";
import { useIntelligenceCorrection } from "@/hooks/useIntelligenceCorrection";
import { AccuracyPrompt, type AccuracyPromptOutcome } from "./AccuracyPrompt";
import styles from "./IntelligenceCorrection.module.css";

export interface IntelligenceCorrectionProps {
  /** Entity the AI assessment belongs to. */
  entityId: string;
  /** Always "account" today — the component is account-only per DOS-41. */
  entityType: "account";
  /** Field key the correction targets (e.g. "state_of_play", "health"). */
  field: string;
  /**
   * Interaction variant.
   *
   * "dismiss" (default) — binary Yes / No for claim validation cards
   * (triage cards, divergences, work suggestions).
   *
   * "correct" — three-state Yes / Partially / No for AI narrative summaries
   * and assessment fields. "No" opens an inline editor so the user can
   * provide the corrected value.
   */
  variant?: "dismiss" | "correct";
  /** Stable claim key for suppression tombstones on dismiss. */
  itemKey?: string | null;
  /**
   * Current value of the field — shown as initial text when the "No" editor
   * opens in "correct" variant. Omit for inline claim-card usage.
   */
  currentValue?: string | null;
  /** Extra side effects after a successful confirmation. */
  onConfirmed?: () => void | Promise<void>;
  /** Extra side effects after a successful dismissal ("dismiss" variant). */
  onDismissed?: () => void | Promise<void>;
  /**
   * Called with the corrected value after a successful "No" correction
   * ("correct" variant). Use this to update the field immediately in the
   * UI so the change is visible without waiting for the next enrichment.
   */
  onCorrected?: (correctedValue: string) => void | Promise<void>;
  /** Optional label override (default: "Is this accurate?"). */
  prompt?: string;
  /** Override for the post-submit done copy. */
  doneLabel?: string;
}

type CorrectionMode =
  | "idle"
  | "annotating"   // "Partially" branch: annotation textarea open
  | "editing"      // "No" branch (correct variant): inline editor open
  | "done";

/**
 * Inline correction prompt rendered after an AI assessment.
 *
 * State machine (dismiss variant):
 *   idle → "Is this accurate?" + Yes / No
 *   Yes  → confirmed → done
 *   No   → dismissed → done
 *   done → "Recorded." + Undo
 *
 * State machine (correct variant):
 *   idle       → "Is this accurate?" + Yes / Partially / No
 *   Yes        → confirmed → done
 *   Partially  → annotation textarea → submit → annotated → done
 *   No         → inline editor (prefilled with currentValue) → submit →
 *                corrected → onCorrected callback → done
 *   done       → "Recorded." + Undo
 */
export function IntelligenceCorrection({
  entityId,
  entityType,
  field,
  variant = "dismiss",
  itemKey,
  currentValue,
  onConfirmed,
  onDismissed,
  onCorrected,
  prompt = "Is this accurate?",
  doneLabel = "Recorded.",
}: IntelligenceCorrectionProps) {
  const { submit, submitting, reset } = useIntelligenceCorrection();
  const [mode, setMode] = useState<CorrectionMode>("idle");
  const [annotation, setAnnotation] = useState("");
  const [correctedText, setCorrectedText] = useState(currentValue ?? "");
  const annotationRef = useRef<HTMLTextAreaElement>(null);
  const editorRef = useRef<HTMLTextAreaElement>(null);

  // ── Yes ─────────────────────────────────────────────────────────────────────
  const onYes = useCallback(async (): Promise<AccuracyPromptOutcome> => {
    const ok = await submit({ entityId, entityType, field, action: "confirmed" });
    if (ok) {
      await onConfirmed?.();
      setMode("done");
      return "done";
    }
    return "stay";
  }, [entityId, entityType, field, onConfirmed, submit]);

  // ── No (dismiss variant) ────────────────────────────────────────────────────
  const onNoDismiss = useCallback(async (): Promise<AccuracyPromptOutcome> => {
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "dismissed",
      itemKey,
    });
    if (ok) {
      await onDismissed?.();
      return "done";
    }
    return "stay";
  }, [entityId, entityType, field, itemKey, onDismissed, submit]);

  // ── Partially (correct variant) ─────────────────────────────────────────────
  const handleOpenAnnotation = useCallback(() => {
    setAnnotation("");
    setMode("annotating");
    // Focus textarea on next tick after render
    requestAnimationFrame(() => annotationRef.current?.focus());
  }, []);

  const handleSubmitAnnotation = useCallback(async () => {
    if (!annotation.trim()) return;
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "annotated",
      annotation: annotation.trim(),
    });
    if (ok) {
      setMode("done");
    }
  }, [annotation, entityId, entityType, field, submit]);

  // ── No (correct variant) ─────────────────────────────────────────────────────
  const handleOpenEditor = useCallback(() => {
    setCorrectedText(currentValue ?? "");
    setMode("editing");
    requestAnimationFrame(() => editorRef.current?.focus());
  }, [currentValue]);

  const handleSubmitCorrection = useCallback(async () => {
    const trimmed = correctedText.trim();
    if (!trimmed) return;
    const ok = await submit({
      entityId,
      entityType,
      field,
      action: "corrected",
      correctedValue: trimmed,
    });
    if (ok) {
      await onCorrected?.(trimmed);
      setMode("done");
    }
  }, [correctedText, entityId, entityType, field, onCorrected, submit]);

  const handleUndo = useCallback(() => {
    reset();
    setMode("idle");
    setAnnotation("");
    setCorrectedText(currentValue ?? "");
  }, [currentValue, reset]);

  // ── Render: done ─────────────────────────────────────────────────────────────
  if (mode === "done") {
    return (
      <span className={styles.wrapper}>
        <span className={styles.done}>
          {doneLabel}{" "}
          <button type="button" className={styles.linkButton} onClick={handleUndo}>
            Undo
          </button>
        </span>
      </span>
    );
  }

  // ── Render: annotating (Partially branch) ────────────────────────────────────
  if (mode === "annotating") {
    return (
      <span className={styles.editor}>
        <span className={styles.editorLabel}>Add context — what did the AI miss?</span>
        <textarea
          ref={annotationRef}
          className={styles.textarea}
          rows={3}
          value={annotation}
          onChange={(e) => setAnnotation(e.target.value)}
          placeholder="e.g. The renewal risk is higher than shown — stakeholder went quiet last week."
          disabled={submitting}
        />
        <span className={styles.editorActions}>
          <button
            type="button"
            className={styles.primaryButton}
            onClick={handleSubmitAnnotation}
            disabled={submitting || !annotation.trim()}
          >
            {submitting ? "Saving…" : "Save note"}
          </button>
          <button
            type="button"
            className={styles.linkButton}
            onClick={() => setMode("idle")}
            disabled={submitting}
          >
            Cancel
          </button>
        </span>
      </span>
    );
  }

  // ── Render: editing (No branch for correct variant) ──────────────────────────
  if (mode === "editing") {
    return (
      <span className={styles.editor}>
        <span className={styles.editorLabel}>What should it say instead?</span>
        <textarea
          ref={editorRef}
          className={styles.textarea}
          rows={4}
          value={correctedText}
          onChange={(e) => setCorrectedText(e.target.value)}
          placeholder="Enter the corrected value…"
          disabled={submitting}
        />
        <span className={styles.editorActions}>
          <button
            type="button"
            className={styles.primaryButton}
            onClick={handleSubmitCorrection}
            disabled={submitting || !correctedText.trim()}
          >
            {submitting ? "Saving…" : "Save correction"}
          </button>
          <button
            type="button"
            className={styles.linkButton}
            onClick={() => setMode("idle")}
            disabled={submitting}
          >
            Cancel
          </button>
        </span>
      </span>
    );
  }

  // ── Render: idle ─────────────────────────────────────────────────────────────
  if (variant === "correct") {
    // Three-state model for AI summary fields
    return (
      <span className={styles.wrapper}>
        <span className={styles.prompt}>{prompt}</span>
        <span className={styles.choices}>
          <button
            type="button"
            className={styles.choice}
            onClick={() => { void onYes(); }}
            disabled={submitting}
          >
            Yes
          </button>
          <button
            type="button"
            className={styles.choice}
            onClick={handleOpenAnnotation}
            disabled={submitting}
          >
            Partially
          </button>
          <button
            type="button"
            className={styles.choice}
            onClick={handleOpenEditor}
            disabled={submitting}
          >
            No
          </button>
        </span>
      </span>
    );
  }

  // Binary model for claim cards (dismiss variant) — delegates to AccuracyPrompt
  // so the triage/divergence/work-suggestion surfaces stay visually consistent.
  return (
    <AccuracyPrompt
      prompt={prompt}
      doneLabel={doneLabel}
      submitting={submitting}
      onUndo={reset}
      onYes={onYes}
      onNo={async (): Promise<AccuracyPromptOutcome> => {
        return await onNoDismiss();
      }}
    />
  );
}
