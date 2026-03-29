import { useCallback, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import s from "./AddToRecord.module.css";

interface AddToRecordProps {
  onAdd: (title: string, content: string) => void;
}

export function AddToRecord({ onAdd }: AddToRecordProps) {
  const [expanded, setExpanded] = useState(false);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const contentRef = useRef<HTMLTextAreaElement>(null);

  const reset = useCallback(() => {
    setTitle("");
    setContent("");
    setExpanded(false);
  }, []);

  const handleSubmit = useCallback(() => {
    const trimmed = title.trim();
    if (!trimmed) return;
    onAdd(trimmed, content.trim());
    reset();
  }, [title, content, onAdd, reset]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        reset();
        return;
      }

      if (e.key === "Enter" && (e.target as HTMLElement).tagName === "INPUT") {
        e.preventDefault();
        contentRef.current?.focus();
        return;
      }

      if (
        e.key === "Enter" &&
        (e.metaKey || e.ctrlKey) &&
        (e.target as HTMLElement).tagName === "TEXTAREA"
      ) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [reset, handleSubmit],
  );

  if (!expanded) {
    return (
      <Button variant="outline" size="sm" className={s.addButton} onClick={() => setExpanded(true)}>
        + Add note
      </Button>
    );
  }

  return (
    <div className={s.form} onKeyDown={handleKeyDown}>
      <input
        className={s.titleInput}
        type="text"
        placeholder="Note title"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        autoFocus
      />
      <textarea
        ref={contentRef}
        className={s.contentTextarea}
        placeholder="Details (optional)"
        rows={3}
        value={content}
        onChange={(e) => setContent(e.target.value)}
      />
      <div className={s.buttons}>
        <Button
          variant="outline"
          size="sm"
          className={s.submitButton}
          disabled={!title.trim()}
          onClick={handleSubmit}
        >
          Add
        </Button>
        <Button variant="ghost" size="sm" className={s.cancelButton} onClick={reset}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
