/**
 * StateBlock — colored left-border callout items.
 * Used in State of Play chapter for "Working" and "Struggling" sections.
 * Each item gets a 3px left border in the label color with padding.
 *
 * I261: Optional onItemChange prop wraps items in EditableText for inline editing.
 * I550: Per-item dismiss (x) and feedback (thumbs up/down) controls.
 */
import { X } from "lucide-react";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import styles from "./StateBlock.module.css";

interface StateBlockProps {
  label: string;
  items: string[];
  labelColor?: string;
  /** When provided, items become click-to-edit. Called with (index, newValue). */
  onItemChange?: (index: number, value: string) => void;
  /** Called when user dismisses an item. */
  onItemDismiss?: (index: number) => void;
  /** Per-item feedback value getter. */
  getItemFeedback?: (index: number) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (index: number, type: "positive" | "negative") => void;
}

export function StateBlock({
  label,
  items,
  labelColor = "var(--color-text-tertiary)",
  onItemChange,
  onItemDismiss,
  getItemFeedback,
  onItemFeedback,
}: StateBlockProps) {
  if (items.length === 0) return null;

  return (
    <div
      className={styles.container}
      style={{ "--state-block-color": labelColor } as React.CSSProperties}
    >
      <div className={styles.label}>{label}</div>
      <div className={styles.itemList}>
        {items.map((item, i) => (
          <div key={i} className={styles.itemRow}>
            <div className={styles.itemContent}>
              {onItemChange ? (
                <EditableText
                  value={item}
                  onChange={(v) => onItemChange(i, v)}
                  as="p"
                  multiline
                  className={styles.itemText}
                />
              ) : (
                <p className={styles.itemText}>{item}</p>
              )}
            </div>
            {(onItemFeedback || onItemDismiss) && (
              <div className={styles.actions}>
                {onItemFeedback && getItemFeedback && (
                  <IntelligenceFeedback
                    value={getItemFeedback(i)}
                    onFeedback={(type) => onItemFeedback(i, type)}
                  />
                )}
                {onItemDismiss && (
                  <button
                    type="button"
                    onClick={() => onItemDismiss(i)}
                    title="Dismiss"
                    className={styles.dismissButton}
                  >
                    <X size={13} />
                  </button>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
