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
    <div style={{ marginBottom: 32 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: labelColor,
          marginBottom: 14,
        }}
      >
        {label}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {items.map((item, i) => (
          <div
            key={i}
            className="state-item-row"
            style={{
              borderLeft: `3px solid ${labelColor}`,
              paddingLeft: 16,
              paddingTop: 4,
              paddingBottom: 4,
              display: "flex",
              alignItems: "flex-start",
              gap: 8,
            }}
          >
            <div style={{ flex: 1, minWidth: 0 }}>
              {onItemChange ? (
                <EditableText
                  value={item}
                  onChange={(v) => onItemChange(i, v)}
                  as="p"
                  multiline
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    lineHeight: 1.65,
                    color: "var(--color-text-primary)",
                    maxWidth: 620,
                    margin: 0,
                  }}
                />
              ) : (
                <p
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    lineHeight: 1.65,
                    color: "var(--color-text-primary)",
                    maxWidth: 620,
                    margin: 0,
                  }}
                >
                  {item}
                </p>
              )}
            </div>
            {(onItemFeedback || onItemDismiss) && (
              <div
                className="state-item-actions"
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 2,
                  flexShrink: 0,
                  marginTop: 2,
                  opacity: 0,
                  transition: "opacity 0.15s ease",
                }}
              >
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
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      justifyContent: "center",
                      width: 22,
                      height: 22,
                      padding: 0,
                      border: "none",
                      borderRadius: 2,
                      background: "transparent",
                      color: "var(--color-text-tertiary)",
                      cursor: "pointer",
                    }}
                  >
                    <X size={13} />
                  </button>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
      <style>{`
        .state-item-row:hover .state-item-actions {
          opacity: 1 !important;
        }
        .state-item-actions:focus-within {
          opacity: 1 !important;
        }
      `}</style>
    </div>
  );
}
