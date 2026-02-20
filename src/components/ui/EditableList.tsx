/**
 * EditableList — Ordered list editing with drag-reorder.
 *
 * Renders a list of inline-editable items with:
 * - Grip handle for drag-to-reorder
 * - Click-to-edit text on each item
 * - Add/remove item controls
 * - Tauri event emission on reorder
 */
import { useState, useRef, useCallback } from "react";
import { emit } from "@tauri-apps/api/event";
import { EditableText } from "./EditableText";
import styles from "./EditableText.module.css";

interface EditableListProps {
  /** Current list items */
  items: string[];
  /** Called when items change (edit, reorder, add, remove) */
  onChange: (items: string[]) => void;
  /** Optional field identifier for Tauri event emission */
  fieldId?: string;
  /** Placeholder for new item input */
  placeholder?: string;
  /** Label shown above the list */
  label?: string;
  /** Style for each item's text */
  itemStyle?: React.CSSProperties;
  /** Whether items are multiline (default: false for list items) */
  multiline?: boolean;
}

export function EditableList({
  items,
  onChange,
  fieldId,
  placeholder = "Add item...",
  label,
  itemStyle,
  multiline = false,
}: EditableListProps) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);
  const [addingItem, setAddingItem] = useState(false);
  const [newItemText, setNewItemText] = useState("");
  const addInputRef = useRef<HTMLInputElement>(null);

  const handleDragStart = useCallback((idx: number) => {
    setDragIndex(idx);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, idx: number) => {
    e.preventDefault();
    setDragOverIndex(idx);
  }, []);

  const handleDrop = useCallback(
    (targetIdx: number) => {
      if (dragIndex == null || dragIndex === targetIdx) {
        setDragIndex(null);
        setDragOverIndex(null);
        return;
      }
      const updated = [...items];
      const [moved] = updated.splice(dragIndex, 1);
      updated.splice(targetIdx, 0, moved);
      onChange(updated);
      emit("editable-list:reorder", {
        fieldId,
        fromIndex: dragIndex,
        toIndex: targetIdx,
      }).catch(() => {});
      setDragIndex(null);
      setDragOverIndex(null);
    },
    [dragIndex, items, onChange, fieldId],
  );

  const handleDragEnd = useCallback(() => {
    setDragIndex(null);
    setDragOverIndex(null);
  }, []);

  const handleItemChange = useCallback(
    (idx: number, value: string) => {
      const updated = [...items];
      updated[idx] = value;
      onChange(updated);
    },
    [items, onChange],
  );

  const handleRemove = useCallback(
    (idx: number) => {
      const updated = items.filter((_, i) => i !== idx);
      onChange(updated);
    },
    [items, onChange],
  );

  const handleAdd = useCallback(() => {
    const trimmed = newItemText.trim();
    if (!trimmed) return;
    onChange([...items, trimmed]);
    setNewItemText("");
    setAddingItem(false);
  }, [items, onChange, newItemText]);

  return (
    <div>
      {label && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "var(--color-text-tertiary)",
            marginBottom: 8,
          }}
        >
          {label}
        </div>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
        {items.map((item, idx) => (
          <div
            key={idx}
            draggable
            onDragStart={() => handleDragStart(idx)}
            onDragOver={(e) => handleDragOver(e, idx)}
            onDrop={() => handleDrop(idx)}
            onDragEnd={handleDragEnd}
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 8,
              padding: "6px 0",
              borderBottom: idx < items.length - 1 ? "1px solid var(--color-rule-light)" : "none",
              opacity: dragIndex === idx ? 0.4 : 1,
              background: dragOverIndex === idx && dragIndex !== idx ? "rgba(228, 172, 60, 0.06)" : "transparent",
              transition: "opacity 0.15s ease, background 0.15s ease",
            }}
          >
            {/* Grip handle */}
            <span
              style={{
                cursor: "grab",
                color: "var(--color-text-tertiary)",
                fontSize: 14,
                lineHeight: 1,
                flexShrink: 0,
                marginTop: 4,
                userSelect: "none",
              }}
              title="Drag to reorder"
            >
              &#x2630;
            </span>

            {/* Editable text */}
            <div style={{ flex: 1, minWidth: 0 }}>
              <EditableText
                value={item}
                onChange={(v) => handleItemChange(idx, v)}
                multiline={multiline}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                  lineHeight: 1.5,
                  ...itemStyle,
                }}
                fieldId={fieldId ? `${fieldId}[${idx}]` : undefined}
              />
            </div>

            {/* Remove button */}
            <button
              onClick={() => handleRemove(idx)}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                color: "var(--color-text-tertiary)",
                fontSize: 16,
                lineHeight: 1,
                flexShrink: 0,
                padding: 0,
                marginTop: 2,
                opacity: 0.5,
                transition: "opacity 0.15s ease",
              }}
              title="Remove item"
              onMouseEnter={(e) => { e.currentTarget.style.opacity = "1"; }}
              onMouseLeave={(e) => { e.currentTarget.style.opacity = "0.5"; }}
            >
              &times;
            </button>
          </div>
        ))}
      </div>

      {/* Add item */}
      {addingItem ? (
        <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
          <input
            ref={addInputRef}
            value={newItemText}
            onChange={(e) => setNewItemText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAdd();
              if (e.key === "Escape") {
                setNewItemText("");
                setAddingItem(false);
              }
            }}
            onBlur={() => {
              if (newItemText.trim()) handleAdd();
              else setAddingItem(false);
            }}
            autoFocus
            placeholder={placeholder}
            style={{
              flex: 1,
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
              background: "none",
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              outline: "none",
              padding: "4px 0",
            }}
          />
        </div>
      ) : (
        <button
          onClick={() => {
            setAddingItem(true);
            requestAnimationFrame(() => addInputRef.current?.focus());
          }}
          className={styles.editable}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "6px 0 0",
            textTransform: "uppercase",
            letterSpacing: "0.06em",
          }}
        >
          + Add
        </button>
      )}
    </div>
  );
}
