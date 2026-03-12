/**
 * EditableInline — Click-to-edit short inline text.
 *
 * Renders as a clickable span. On click, becomes a single-line input.
 * On blur or Enter, commits. Escape cancels.
 */
import { useState, useEffect, useRef } from "react";
import styles from "./editable-inline.module.css";

interface EditableInlineProps {
  value: string;
  onSave: (v: string) => void;
  placeholder?: string;
}

export function EditableInline({
  value,
  onSave,
  placeholder,
}: EditableInlineProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  function commit() {
    setEditing(false);
    if (draft.trim() !== value) {
      onSave(draft.trim());
    }
  }

  if (editing) {
    return (
      <input
        ref={inputRef}
        type="text"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
        }}
        placeholder={placeholder}
        className={styles.input}
      />
    );
  }

  return (
    <span
      onClick={() => setEditing(true)}
      className={styles.display}
    >
      {value ? (
        <span>{value}</span>
      ) : (
        <span className={styles.placeholder}>
          {placeholder ?? "Add"}
        </span>
      )}
    </span>
  );
}
