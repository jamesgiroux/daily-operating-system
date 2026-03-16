/**
 * EditableTextarea — Click-to-edit multiline text.
 *
 * Renders as a clickable div with prose styling. On click, becomes a textarea.
 * On blur, commits. Escape cancels.
 */
import { useState, useEffect, useRef } from "react";
import styles from "./editable-textarea.module.css";

interface EditableTextareaProps {
  value: string;
  onSave: (v: string) => void;
  placeholder?: string;
}

export function EditableTextarea({
  value,
  onSave,
  placeholder,
}: EditableTextareaProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (editing) textareaRef.current?.focus();
  }, [editing]);

  function commit() {
    setEditing(false);
    if (draft.trim() !== value) {
      onSave(draft.trim());
    }
  }

  if (editing) {
    return (
      <textarea
        ref={textareaRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
        }}
        placeholder={placeholder}
        rows={4}
        className={styles.textarea}
      />
    );
  }

  return (
    <div
      onClick={() => setEditing(true)}
      className={styles.display}
    >
      {value ? (
        <p className={styles.displayText}>{value}</p>
      ) : (
        <p className={styles.placeholder}>
          {placeholder ?? "Click to add..."}
        </p>
      )}
    </div>
  );
}
