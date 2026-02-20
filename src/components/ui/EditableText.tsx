/**
 * EditableText — Click-to-edit inline text.
 *
 * Renders as a normal text element. On hover, shows a subtle background
 * hint. On click, becomes a textarea (default) or input with matched styling.
 * On blur or Enter (single-line), commits the change.
 * Escape cancels without saving.
 *
 * Keyboard navigation:
 * - Tab: commit and focus next editable element
 * - Shift+Tab: commit and focus previous editable element
 * - Escape: cancel edit
 * - Enter (single-line only): commit
 *
 * Emits a Tauri "editable-text:commit" event on save for persistence layer.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { emit } from "@tauri-apps/api/event";
import styles from "./EditableText.module.css";

interface EditableTextProps {
  /** Current text value */
  value: string;
  /** Called when user commits an edit (blur or Enter) */
  onChange: (value: string) => void;
  /** HTML element to render in display mode */
  as?: "span" | "p" | "h1" | "h2" | "div";
  /** Inline styles applied to both display and edit mode */
  style?: React.CSSProperties;
  /**
   * Use textarea instead of input (for multi-sentence text).
   * @default true — textarea is the default mode.
   * Set to false for single-line inputs.
   */
  multiline?: boolean;
  /** Placeholder when value is empty */
  placeholder?: string;
  /** Optional field identifier for Tauri event emission */
  fieldId?: string;
}

/** CSS data attribute used to identify editable elements for Tab navigation */
const EDITABLE_ATTR = "data-editable-text";

/** Find the next (or previous) editable element in document order */
function findSiblingEditable(current: Element, direction: "next" | "prev"): HTMLElement | null {
  const all = Array.from(document.querySelectorAll(`[${EDITABLE_ATTR}]`));
  const idx = all.indexOf(current);
  if (idx === -1) return null;
  const target = direction === "next" ? all[idx + 1] : all[idx - 1];
  return (target as HTMLElement) ?? null;
}

export function EditableText({
  value,
  onChange,
  as: Tag = "span",
  style,
  multiline = true,
  placeholder,
  fieldId,
}: EditableTextProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLTextAreaElement | HTMLInputElement>(null);
  const wrapperRef = useRef<HTMLElement>(null);

  // Sync draft when external value changes (e.g. regenerate)
  useEffect(() => {
    if (!editing) setDraft(value);
  }, [value, editing]);

  // Focus + select on enter edit mode
  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  const commit = useCallback(() => {
    setEditing(false);
    const trimmed = draft.trim();
    if (trimmed !== value) {
      onChange(trimmed);
      // Fire Tauri event for persistence layer
      emit("editable-text:commit", {
        fieldId: fieldId ?? undefined,
        value: trimmed,
        previousValue: value,
      }).catch(() => {});
    }
  }, [draft, value, onChange, fieldId]);

  const cancel = useCallback(() => {
    setDraft(value);
    setEditing(false);
  }, [value]);

  // Auto-resize textarea to content
  const autoResize = useCallback((el: HTMLTextAreaElement) => {
    el.style.height = "auto";
    el.style.height = el.scrollHeight + "px";
  }, []);

  /** Shared keyboard handler for both input and textarea */
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        cancel();
        return;
      }

      // Tab navigation: commit and move to sibling editable
      if (e.key === "Tab") {
        e.preventDefault();
        commit();
        const direction = e.shiftKey ? "prev" : "next";
        const wrapper = wrapperRef.current ?? inputRef.current?.closest(`[${EDITABLE_ATTR}]`);
        if (wrapper) {
          const sibling = findSiblingEditable(wrapper, direction);
          if (sibling) {
            sibling.click();
          }
        }
        return;
      }

      // Enter commits in single-line mode only
      if (e.key === "Enter" && !multiline) {
        commit();
      }
    },
    [cancel, commit, multiline],
  );

  if (editing) {
    const inputStyle: React.CSSProperties = {
      ...style,
      background: "transparent",
      border: "none",
      borderBottom: "2px solid var(--color-spice-terracotta)",
      outline: "none",
      width: "100%",
      resize: "none",
      padding: 0,
      margin: style?.margin ?? 0,
      boxSizing: "border-box",
    };

    if (multiline) {
      return (
        <textarea
          ref={inputRef as React.RefObject<HTMLTextAreaElement>}
          value={draft}
          onChange={(e) => {
            setDraft(e.target.value);
            autoResize(e.target);
          }}
          onBlur={commit}
          onKeyDown={handleKeyDown}
          onFocus={(e) => autoResize(e.target)}
          style={inputStyle}
          rows={2}
          {...{ [EDITABLE_ATTR]: "" }}
        />
      );
    }

    return (
      <input
        ref={inputRef as React.RefObject<HTMLInputElement>}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={handleKeyDown}
        style={inputStyle}
        {...{ [EDITABLE_ATTR]: "" }}
      />
    );
  }

  return (
    <Tag
      ref={wrapperRef as unknown as React.Ref<HTMLDivElement>}
      onClick={() => setEditing(true)}
      className={styles.editable}
      style={style}
      title="Click to edit"
      {...{ [EDITABLE_ATTR]: "" }}
    >
      {value || placeholder}
    </Tag>
  );
}
