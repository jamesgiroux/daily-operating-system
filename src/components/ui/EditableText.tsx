/**
 * EditableText — Click-to-edit inline text.
 *
 * Renders as a normal text element. On hover, shows a subtle background
 * hint. On click, becomes an input/textarea with matched styling.
 * On blur or Enter (single-line), commits the change.
 * Escape cancels without saving.
 */
import { useState, useEffect, useRef, useCallback } from "react";
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
  /** Use textarea instead of input (for multi-sentence text) */
  multiline?: boolean;
  /** Placeholder when value is empty */
  placeholder?: string;
}

export function EditableText({
  value,
  onChange,
  as: Tag = "span",
  style,
  multiline = false,
  placeholder,
}: EditableTextProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLTextAreaElement | HTMLInputElement>(null);

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
    }
  }, [draft, value, onChange]);

  const cancel = useCallback(() => {
    setDraft(value);
    setEditing(false);
  }, [value]);

  // Auto-resize textarea to content
  const autoResize = useCallback((el: HTMLTextAreaElement) => {
    el.style.height = "auto";
    el.style.height = el.scrollHeight + "px";
  }, []);

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
          onKeyDown={(e) => {
            if (e.key === "Escape") cancel();
          }}
          onFocus={(e) => autoResize(e.target)}
          style={inputStyle}
          rows={2}
        />
      );
    }

    return (
      <input
        ref={inputRef as React.RefObject<HTMLInputElement>}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") cancel();
        }}
        style={inputStyle}
      />
    );
  }

  return (
    <Tag
      onClick={() => setEditing(true)}
      className={styles.editable}
      style={style}
      title="Click to edit"
    >
      {value || placeholder}
    </Tag>
  );
}
