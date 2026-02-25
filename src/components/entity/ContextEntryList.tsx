import { useState } from "react";
import { formatShortDate } from "@/lib/utils";
import s from "./ContextEntryList.module.css";

interface ContextEntryListProps {
  entries: { id: string; title: string; content: string; createdAt: string }[];
  onUpdate: (id: string, title: string, content: string) => void;
  onDelete: (id: string) => void;
  onCreate: (title: string, content: string) => void;
  addLabel?: string;
  placeholders?: { title?: string; content?: string };
}

export function ContextEntryList({
  entries,
  onUpdate,
  onDelete,
  onCreate,
  addLabel = "+ Add context entry",
  placeholders,
}: ContextEntryListProps) {
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newContent, setNewContent] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editContent, setEditContent] = useState("");

  const handleCreate = () => {
    if (!newTitle.trim() || !newContent.trim()) return;
    onCreate(newTitle.trim(), newContent.trim());
    setNewTitle("");
    setNewContent("");
    setAdding(false);
  };

  const startEdit = (entry: { id: string; title: string; content: string }) => {
    setEditingId(entry.id);
    setEditTitle(entry.title);
    setEditContent(entry.content);
  };

  const commitEdit = () => {
    if (editingId && editTitle.trim() && editContent.trim()) {
      onUpdate(editingId, editTitle.trim(), editContent.trim());
    }
    setEditingId(null);
  };

  return (
    <div>
      <div className={s.entryList}>
        {entries.map((entry) =>
          editingId === entry.id ? (
            <div key={entry.id} className={s.editEntryForm}>
              <input
                className={s.addEntryInput}
                value={editTitle}
                onChange={(e) => setEditTitle(e.target.value)}
                placeholder="Title"
                autoFocus
              />
              <textarea
                className={s.addEntryTextarea}
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                placeholder="Content"
                rows={3}
              />
              <div className={s.addEntryActions}>
                <button className={s.addEntryCancel} onClick={() => setEditingId(null)}>
                  Cancel
                </button>
                <button className={s.addEntrySave} onClick={commitEdit}>
                  Save
                </button>
              </div>
            </div>
          ) : (
            <div key={entry.id} className={s.entryItem}>
              <div className={s.entryHeader}>
                <span className={s.entryTitle}>{entry.title}</span>
                <span className={s.entryDate}>{formatShortDate(entry.createdAt)}</span>
              </div>
              <div className={s.entryContent}>{entry.content}</div>
              <div className={s.entryActions}>
                <button className={s.entryActionBtn} onClick={() => startEdit(entry)}>
                  Edit
                </button>
                <button className={s.entryActionBtn} onClick={() => onDelete(entry.id)}>
                  Delete
                </button>
              </div>
            </div>
          ),
        )}
      </div>

      {adding ? (
        <div className={s.addEntryForm}>
          <input
            className={s.addEntryInput}
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder={placeholders?.title ?? "e.g., 'Infrastructure scaling philosophy' or 'Customer success metrics'"}
            autoFocus
          />
          <textarea
            className={s.addEntryTextarea}
            value={newContent}
            onChange={(e) => setNewContent(e.target.value)}
            placeholder={placeholders?.content ?? "Write 1\u20133 paragraphs about your approach, methodology, or key insight."}
            rows={3}
          />
          <div className={s.addEntryActions}>
            <button
              className={s.addEntryCancel}
              onClick={() => {
                setAdding(false);
                setNewTitle("");
                setNewContent("");
              }}
            >
              Cancel
            </button>
            <button className={s.addEntrySave} onClick={handleCreate}>
              Save
            </button>
          </div>
        </div>
      ) : (
        <button className={s.addBtn} onClick={() => setAdding(true)}>
          {addLabel}
        </button>
      )}
    </div>
  );
}
