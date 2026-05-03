/**
 * @deprecated Use `IntelligenceCorrection` on account pages instead.
 * `IntelligenceFeedback` remains on project and person pages during the
 * strangler-fig migration. Do not add new usages on account surfaces.
 */
import { ThumbsUp, ThumbsDown } from "lucide-react";
import styles from "./IntelligenceFeedback.module.css";

interface IntelligenceFeedbackProps {
  /** Current vote state for this field */
  value: "positive" | "negative" | null;
  /** Called when user clicks a thumb */
  onFeedback: (type: "positive" | "negative") => void;
}

/**
 * Inline thumbs-up / thumbs-down feedback for AI-generated intelligence.
 *
 * Renders two small icon buttons. Hidden by default (opacity: 0),
 * revealed on hover or when a vote is active.
 *
 * Usage:
 * ```tsx
 * <IntelligenceFeedback
 *   value={getFeedback("risks[0]")}
 *   onFeedback={(type) => submitFeedback("risks[0]", type)}
 * />
 * ```
 */
export function IntelligenceFeedback({
  value,
  onFeedback,
}: IntelligenceFeedbackProps) {
  const hasVote = value !== null;

  return (
    <span className={styles.wrapper}>
      <span
        className={`${styles.container}${hasVote ? ` ${styles.visible}` : ""}`}
      >
        <button
          type="button"
          className={`${styles.button}${value === "positive" ? ` ${styles.positive}` : ""}`}
          onClick={(e) => {
            e.stopPropagation();
            onFeedback("positive");
          }}
          aria-label="This was helpful"
          aria-pressed={value === "positive"}
          title="Helpful"
        >
          <ThumbsUp size={13} />
        </button>
        <button
          type="button"
          className={`${styles.button}${value === "negative" ? ` ${styles.negative}` : ""}`}
          onClick={(e) => {
            e.stopPropagation();
            onFeedback("negative");
          }}
          aria-label="This was not helpful"
          aria-pressed={value === "negative"}
          title="Not helpful"
        >
          <ThumbsDown size={13} />
        </button>
      </span>
    </span>
  );
}
