import { useCallback, useMemo, useState } from "react";
import styles from "./AccuracyPrompt.module.css";

export type AccuracyPromptOutcome = "done" | "reset" | "stay";

interface AccuracyPromptProps {
  prompt?: string;
  yesLabel?: string;
  noLabel?: string;
  presentation?: "inline" | "meta";
  revealOnClick?: boolean;
  doneLabel?: string | null;
  submitting?: boolean;
  onYes: () => Promise<AccuracyPromptOutcome> | AccuracyPromptOutcome;
  onNo: () => Promise<AccuracyPromptOutcome> | AccuracyPromptOutcome;
  onUndo?: () => void;
}

type Mode = "collapsed" | "expanded" | "done";

export function AccuracyPrompt({
  prompt = "Is this accurate?",
  yesLabel = "Yes",
  noLabel = "No",
  presentation = "inline",
  revealOnClick = false,
  doneLabel = null,
  submitting = false,
  onYes,
  onNo,
  onUndo,
}: AccuracyPromptProps) {
  const initialMode: Mode = revealOnClick ? "collapsed" : "expanded";
  const [mode, setMode] = useState<Mode>(initialMode);

  const reset = useCallback(() => {
    setMode(initialMode);
  }, [initialMode]);

  const handleResult = useCallback(
    (result: AccuracyPromptOutcome) => {
      if (result === "done") {
        if (doneLabel) setMode("done");
        else reset();
        return;
      }
      if (result === "reset") {
        reset();
      }
    },
    [doneLabel, reset],
  );

  const runYes = useCallback(async () => {
    const result = await onYes();
    handleResult(result);
  }, [handleResult, onYes]);

  const runNo = useCallback(async () => {
    const result = await onNo();
    handleResult(result);
  }, [handleResult, onNo]);

  const choiceClass = useMemo(
    () => (presentation === "meta" ? styles.choiceMeta : styles.choiceInline),
    [presentation],
  );

  if (mode === "done" && doneLabel) {
    return (
      <span className={styles.wrapper}>
        <span className={styles.done}>
          {doneLabel}{" "}
          {onUndo ? (
            <button type="button" className={styles.linkButton} onClick={() => { onUndo(); reset(); }}>
              Undo
            </button>
          ) : null}
        </span>
      </span>
    );
  }

  if (mode === "collapsed") {
    return (
      <button
        type="button"
        className={styles.metaTrigger}
        onClick={() => setMode("expanded")}
      >
        {prompt}
      </button>
    );
  }

  return (
    <span className={styles.wrapper}>
      {presentation === "meta" ? (
        <span className={styles.metaPrompt}>
          <span className={styles.metaPromptLabel}>{prompt}</span>
        </span>
      ) : (
        <span className={styles.inlinePrompt}>{prompt}</span>
      )}
      <span className={styles.choices}>
        <button
          type="button"
          className={choiceClass}
          onClick={runYes}
          disabled={submitting}
        >
          {yesLabel}
        </button>
        <button
          type="button"
          className={choiceClass}
          onClick={runNo}
          disabled={submitting}
        >
          {noLabel}
        </button>
      </span>
    </span>
  );
}
